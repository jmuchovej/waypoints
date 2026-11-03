//! The module backend: the compiled plugin under `wasmi`, instantiated with
//! an empty linker, driven through the real exports.

use std::collections::BTreeMap;

use domain::Json;
use wasmi::{Engine, ExternType, Instance, Linker, Memory, Module, Store, WasmParams, WasmResults};

use super::{
    error::HarnessError,
    native::{check_config, unwrap},
    run::{Entry, Run},
};
use crate::abi::{ABI, FinalizeOutput, Init, Manifest, Output};

/// The export set and signatures a plugin module has, and nothing else.
const SHAPE: [(&str, &str); 9] = [
    ("memory", "memory"),
    ("wp_abi", "() -> i32"),
    ("wp_alloc", "(i32) -> i32"),
    ("wp_entry", "(i32, i32, i32, i32, i32) -> i64"),
    ("wp_finalize", "(i32) -> i64"),
    ("wp_free", "(i32, i32) -> ()"),
    ("wp_init", "(i32, i32) -> i64"),
    ("wp_manifest", "() -> i64"),
    ("wp_panic", "() -> i64"),
];

/// A loaded, instantiated plugin module.
pub struct Plugin {
    store: Store<()>,
    instance: Instance,
    memory: Memory,
}

impl Plugin {
    /// Load a module and check it is a plugin: it imports nothing, exports
    /// exactly the ABI's shape, instantiates with an empty linker, and
    /// speaks ABI [`ABI`].
    pub fn load(bytes: &[u8]) -> Result<Self, HarnessError> {
        let engine = Engine::default();
        let module =
            Module::new(&engine, bytes).map_err(|error| HarnessError::Module(error.to_string()))?;

        let imports: Vec<String> = module
            .imports()
            .map(|import| format!("{}.{}", import.module(), import.name()))
            .collect();
        if !imports.is_empty() {
            return Err(HarnessError::Imports(imports));
        }

        let found: BTreeMap<String, String> = module
            .exports()
            .map(|export| (export.name().to_string(), describe(export.ty())))
            .collect();
        let expected: BTreeMap<String, String> = SHAPE
            .iter()
            .map(|(name, signature)| (name.to_string(), signature.to_string()))
            .collect();
        if found != expected {
            return Err(HarnessError::Shape { expected, found });
        }

        let mut store = Store::new(&engine, ());
        let instance = Linker::<()>::new(&engine)
            .instantiate_and_start(&mut store, &module)
            .map_err(|error| HarnessError::Module(error.to_string()))?;
        let memory = instance
            .get_memory(&store, "memory")
            .ok_or_else(|| HarnessError::Module("no `memory` export".to_string()))?;
        let mut plugin = Self {
            store,
            instance,
            memory,
        };

        let abi: u32 = plugin.call("wp_abi", ())?;
        if abi != ABI {
            return Err(HarnessError::Abi {
                expected: ABI,
                found: abi,
            });
        }
        Ok(plugin)
    }

    /// Run the plugin over `entries`, in the order given, through the exports.
    pub fn run(&mut self, config: &Json, entries: &[Entry]) -> Result<Run, HarnessError> {
        let packed: u64 = self.call("wp_manifest", ())?;
        let manifest = self.take(packed)?;
        let parsed: Manifest =
            serde_json::from_slice(&manifest).map_err(|error| HarnessError::Payload {
                call: "wp_manifest".to_string(),
                error: error.to_string(),
            })?;
        check_config(&parsed.options, config)?;

        let config = serde_json::to_vec(config).expect("configuration is JSON");
        let (ptr, len) = self.give(&config)?;
        let packed: u64 = self.call("wp_init", (ptr, len))?;
        self.free(ptr, len)?;
        let init = self.take(packed)?;
        let state = unwrap::<Init>("wp_init", &init)?.state;

        let mut outputs = Vec::with_capacity(entries.len());
        for entry in entries {
            let (name_ptr, name_len) = self.give(entry.name.as_bytes())?;
            let (bytes_ptr, bytes_len) = self.give(&entry.bytes)?;
            let packed: u64 = self.call(
                "wp_entry",
                (state, name_ptr, name_len, bytes_ptr, bytes_len),
            )?;
            self.free(name_ptr, name_len)?;
            self.free(bytes_ptr, bytes_len)?;
            let output = self.take(packed)?;
            unwrap::<Output>("wp_entry", &output)?;
            outputs.push((entry.name.clone(), output));
        }

        let packed: u64 = self.call("wp_finalize", state)?;
        let finalize = self.take(packed)?;
        unwrap::<FinalizeOutput>("wp_finalize", &finalize)?;
        Ok(Run {
            manifest,
            entries: outputs,
            finalize,
        })
    }

    /// What `wp_panic` has to say: the last panic's message, if any.
    pub fn panic(&mut self) -> Option<String> {
        let func = self
            .instance
            .get_typed_func::<(), u64>(&self.store, "wp_panic")
            .ok()?;
        let packed = func.call(&mut self.store, ()).ok()?;
        if packed == 0 {
            return None;
        }
        let (ptr, len) = unpack(packed);
        let mut bytes = vec![0; len as usize];
        self.memory
            .read(&self.store, ptr as usize, &mut bytes)
            .ok()?;
        String::from_utf8(bytes).ok()
    }

    /// Call an export; a trap is reported with what `wp_panic` says after it.
    fn call<P: WasmParams, R: WasmResults>(
        &mut self,
        name: &str,
        params: P,
    ) -> Result<R, HarnessError> {
        let func = self
            .instance
            .get_typed_func::<P, R>(&self.store, name)
            .map_err(|error| HarnessError::Module(error.to_string()))?;
        func.call(&mut self.store, params).map_err(|error| {
            let panic = self.panic();
            HarnessError::Trap {
                call: name.to_string(),
                error: error.to_string(),
                panic,
            }
        })
    }

    /// Copy bytes into a guest buffer the guest allocated.
    fn give(&mut self, bytes: &[u8]) -> Result<(u32, u32), HarnessError> {
        let len = u32::try_from(bytes.len())
            .map_err(|_| HarnessError::Module("an entry exceeds 4 GiB".to_string()))?;
        let ptr: u32 = self.call("wp_alloc", len)?;
        self.memory
            .write(&mut self.store, ptr as usize, bytes)
            .map_err(|error| HarnessError::Module(error.to_string()))?;
        Ok((ptr, len))
    }

    /// Copy a packed guest buffer out and release it.
    fn take(&mut self, packed: u64) -> Result<Vec<u8>, HarnessError> {
        let (ptr, len) = unpack(packed);
        let mut bytes = vec![0; len as usize];
        self.memory
            .read(&self.store, ptr as usize, &mut bytes)
            .map_err(|error| HarnessError::Module(error.to_string()))?;
        self.free(ptr, len)?;
        Ok(bytes)
    }

    fn free(&mut self, ptr: u32, len: u32) -> Result<(), HarnessError> {
        self.call::<(u32, u32), ()>("wp_free", (ptr, len))
    }
}

fn unpack(packed: u64) -> (u32, u32) {
    ((packed >> 32) as u32, packed as u32)
}

fn describe(ty: &ExternType) -> String {
    match ty {
        ExternType::Func(func) => {
            let list = |types: &[wasmi::ValType]| {
                types
                    .iter()
                    .map(|ty| format!("{ty:?}").to_lowercase())
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            format!(
                "({}) -> {}",
                list(func.params()),
                match func.results() {
                    [] => "()".to_string(),
                    [one] => format!("{one:?}").to_lowercase(),
                    many => format!("({})", list(many)),
                }
            )
        }
        ExternType::Memory(_) => "memory".to_string(),
        ExternType::Table(_) => "table".to_string(),
        ExternType::Global(_) => "global".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A module whose only content is one function import, `env.f: () -> ()`,
    /// spelled out so the test needs no text-format support in `wasmi`.
    const IMPORTING: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, // magic, version
        0x01, 0x04, 0x01, 0x60, 0x00, 0x00, // type section: one `() -> ()`
        0x02, 0x09, 0x01, 0x03, 0x65, 0x6e, 0x76, 0x01, 0x66, 0x00,
        0x00, // import "env" "f" func 0
    ];

    #[test]
    fn a_module_with_an_import_is_not_a_plugin_and_the_import_is_named() {
        let Err(error) = Plugin::load(IMPORTING) else {
            panic!("an importing module loaded");
        };
        let HarnessError::Imports(imports) = error else {
            panic!("{error}");
        };
        assert_eq!(imports, ["env.f"]);
    }

    #[test]
    fn a_module_without_the_exports_is_not_a_plugin() {
        let empty = &IMPORTING[..8];
        let Err(error) = Plugin::load(empty) else {
            panic!("an empty module loaded");
        };
        let HarnessError::Shape { found, .. } = error else {
            panic!("{error}");
        };
        assert!(found.is_empty());
    }
}
