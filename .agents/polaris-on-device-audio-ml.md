# polaris — On-Device Audio ML Architecture Decision Record

> **Audience:** a local Claude Code agent working in the Waypoints repo.
> **Scope:** the `polaris` pillar — ASR (speech→text), TTS (text→speech), speaker diarization, and voiceprint identification — and the inference-backend choices underneath them.
> **Status:** decisions are marked **[DECIDED]**, **[LEANING]**, or **[OPEN]**. Treat OPEN items as prompts for a design conversation, not settled facts.
> **Date of synthesis:** derived from an April 2026 research session; re-verify model/library specifics before load-bearing use, since this space moves monthly.

---

## 0. Stack premise [RESOLVED 2026-07-06 — Tauri]

The target is **Rust + Tauri (Nuxt 3 / Vue frontend, Rust backend), LibSQL/Turso for persistence**. Confirmed with the user; the Waypoints workspace itself settles it (Tauri workspace member, the cartograph/store crate structure). The Flutter/Dart references in persona/preference material are a generic cross-project preference and do not apply to this repo. Candle and `ort` are Rust crates consumed by the Tauri Rust backend, as assumed throughout. §7–§9 are unblocked.

---

## 1. The three mental models that drive every decision

These are the load-bearing conceptual facts. Internalize them; most concrete choices fall out of them.

### 1.1 NPUs are black boxes; GPUs are not
Consumer NPUs (Apple ANE, Qualcomm Hexagon, Google Edge TPU, Samsung Exynos NPU) are **fixed-function accelerators accessed through vendor-controlled, graph-level APIs** — not programmable devices with an op-level interface like a GPU. You cannot dispatch individual kernels to an NPU. You hand a *whole model graph* to a vendor compiler (CoreML, QNN, Edge TPU compiler), which decides which subgraphs the silicon can execute. There is no public ISA, no shader language, no JIT. This is deliberate: graph-level lock-in lets vendors redesign the silicon between generations (Apple substantially changed the ANE across A-series generations) without breaking the developer contract.

**Consequence:** extracting real NPU performance requires **per-vendor model conversion + per-vendor architectural massaging** to fit each compiler's supported op set. This is a maintenance burden that scales with the number of NPU vendors targeted. It is not a build-time button-press.

### 1.2 "Op-level" frameworks can't reach NPUs; "graph-level" runtimes can
- **Op-level runtimes** (PyTorch eager, **Candle**, MLX-as-array-lib): user code calls tensor ops that dispatch to a backend. Maps cleanly to CPU / CUDA / Metal (all op-level APIs). **Cannot** map to CoreML/NNAPI/QNN (graph-level APIs). → **No NPU access, ever, by design.**
- **Graph-level runtimes** (ONNX Runtime, TFLite/LiteRT): consume a serialized *graph*, and ship pre-built integrations (execution providers / delegates) that hand supported subgraphs to the vendor compilers. → **Partial NPU access, automatically, for free.**

### 1.3 "A model is in candle-transformers" ≠ "the weights were converted"
A common misconception. Two distinct things a framework needs:
- **Weights** — numbered tensor blobs (`safetensors`, `.gguf`, `.pt`). Candle reads these natively; *any* model's weights are already loadable.
- **Architecture** — the `forward()` computation graph. In Candle this is **hand-written Rust** using Candle ops, ported faithfully from the reference (PyTorch/NeMo) implementation, including all the fiddly bits (RoPE, KV cache, attention masks, activation choice, TDT decoding loop, etc.).

"In candle-transformers" means **someone re-implemented the architecture in Rust.** That's a real engineering project even though the weights are trivially loadable. This is why Whisper works in Candle (ported) but Parakeet does not (not ported).

**ONNX breaks this pattern:** an ONNX file serializes *the graph itself* as standardized ops, so a generic ONNX runtime executes it with no per-model architecture code — except any autoregressive decoding loop, which stays in host code. One ONNX file → runs on ONNX Runtime, sherpa-onnx, `ort`, `onnxruntime-web`, etc.

---

## 2. The three-tier deployment strategy [DECIDED framing]

Every audio-ML target sits on this ladder. Each tier roughly 2–3×'s performance and roughly 2×'s engineering cost over the one below. There is no shortcut to a higher tier.

| Tier | Approach | iPhone perf | Snapdragon perf | Cross-platform cost | Effort |
|---|---|---|---|---|---|
| **T1** | Pure Rust (Candle, CPU on mobile) | 1× (CPU) / ~3–5× (Metal on Mac) | 1× (CPU only) | Trivial — one binary | ~5h |
| **T2** | `ort` (ONNX Runtime) + vendor EPs | ~5–10× (partial CoreML/ANE) | ~15–25× (QNN/Hexagon) | Same Rust code, EPs per-platform | ~15h + CI |
| **T3** | Vendor-native, hand-tuned | 30–50× (full ANE via WhisperKit) | maximal | N separate stacks | ~50–100h + perpetual tax |

**[DECIDED] Target Tier 2 (`ort`) as the default for polaris.** It is the Pareto-optimal point for a solo developer on a ~40h/5-week budget: good-enough across all platforms, one Rust orchestration codebase, NPU access without writing Swift/Kotlin. **Tier 1 (Candle-only)** is an acceptable fallback if mobile is deferred and desktop-Mac (Metal) is the only surface that must be fast. **Tier 3** is only justified if polaris becomes the centerpiece product with funding — and even then, adopt it surgically (one platform at a time) via a thin native sidecar.

Waypoints is a research-grade tool for the author + a small community, not an Otter.ai competitor. Do **not** chase Tier-3 peak silicon utilization prematurely.

---

## 3. Backend decision matrix [DECIDED]

| | **Candle** | **`ort` (ONNX Runtime)** | **Vendor-native (WhisperKit / FluidAudio / SpeakerKit)** |
|---|---|---|---|
| Language | Pure Rust | Rust wrapper over C++ lib | Swift (Apple-only) |
| Native lib added | none (static) | one `libonnxruntime` per platform, ~15–25 MB | Swift Package |
| NPU/ANE | ❌ never (Metal GPU max on Mac; CPU on Android) | ⚠️ partial, automatic (CoreML/NNAPI/QNN/OpenVINO EPs) | ✅ full ANE, hand-tuned |
| Model coverage | Whisper, Distil-Whisper, EnCodec, some TTS. **No** Parakeet/Canary/Moonshine/pyannote | Everything with an ONNX export (Whisper, Moonshine, Parakeet-TDT, pyannote, ECAPA-TDNN, Piper, Kokoro) | Whisper + Parakeet + pyannote community-1, pre-optimized |
| Quantization | GGUF/GGML (whisper.cpp-tier), INT4/5/8 | INT8 default, INT4 encoder-only | FP16/INT8 tuned per model |
| WASM/browser | ✅ first-class | ⚠️ via `onnxruntime-web` | ❌ |
| Tauri fit | perfect (crate) | perfect (crate + bundled lib) | Swift sidecar via Tauri iOS plugin FFI |
| Fine-tuning (LoRA) | ✅ `candle-lora` | ❌ inference only | ❌ |

**Rules of thumb for Claude Code:**
- **Primary inference backend → `ort`.** One dependency buys the entire ASR/TTS/diarization/embedding landscape + NPU offload.
- **Keep Candle in the tree for two jobs only:** (a) LoRA fine-tuning on desktop, (b) the WASM/browser dictation target and lightweight CPU fallback.
- **Reach for vendor-native only** if profiling proves polaris is the bottleneck on a specific platform (realistically iOS battery/latency). Then add WhisperKit+SpeakerKit behind the same trait, Apple-only.
- **TFLite/LiteRT** is the Android-native equivalent of CoreML; `ort`'s NNAPI/XNNPACK EPs abstract most of its value, so don't adopt it directly unless QNN-via-`ort` proves insufficient on Snapdragon.

### `ort` dependency footprint (concrete)
```toml
# src-tauri/Cargo.toml — inference deps
ort        = { version = "2", features = ["coreml"] }   # swap EP feature per target
ndarray    = "0.16"   # tensor I/O for ort
tokenizers = "0.20"   # HF tokenizers (same crate regardless of backend)
hf-hub     = "0.3"    # one-time model download, then fully offline
hound      = "3.5"    # WAV I/O
cpal       = "0.15"   # audio capture
symphonia  = "0.5"    # mp3/m4a/flac decode
# optional Candle path:
candle-core = "0.8"; candle-nn = "0.8"; candle-transformers = "0.8"
```
Adds: 1 Rust crate + 1 bundled native lib per platform + per-model ONNX files (80 MB Moonshine-Tiny-INT8 … 1–2 GB Whisper-Large). EP features gate per target: `coreml` (Apple), `qnn`/`xnnpack` (Android), `openvino` (Intel Core Ultra), `cuda` (Linux/Win desktop). Tauri already bundles native libs for desktop; mobile goes to `jniLibs/` (Android) or the iOS static-link step.

**Honest caveat on `ort` + CoreML:** the CoreML EP does best-effort ONNX→CoreML subgraph conversion at load time, not the hand-tuned op-substitution WhisperKit does. Expect ~15% of theoretical ANE peak vs. WhisperKit's ~90% — still meaningfully faster and cooler than CPU, but not WhisperKit-tier. This is the exact seam where a Tier-3 upgrade would later slot in.

---

## 4. Model selection by task

### 4.1 ASR [LEANING]
| Model | License | Best backend on Apple | Cross-platform backend | Use when |
|---|---|---|---|---|
| **Whisper-Large-v3-Turbo** (809M) | **MIT** | WhisperKit (T3) or `ort`+CoreML (T2) | `ort`, sherpa-onnx | Default; hallucination-resistant on noisy/long audio; 99 langs |
| **Parakeet-TDT-0.6B-v3** | **CC-BY-4.0** | FluidAudio (T3) or `ort` | sherpa-onnx | Throughput king; clean/short-form English + 24 EU langs; ~66 MB ANE working set |
| **Moonshine** (Tiny 27M → Medium) | **MIT** | `ort` | **sherpa-onnx** | Edge/phone default; variable-length encoder, tiny, fastest on weak hardware |
| **Distil-Whisper-large-v3.5** (756M) | **MIT** | Candle or `ort` | universal (GGUF/ONNX/MLX) | When you want Whisper quality + speed and a Candle-native path |

**Practical default:** Whisper-Large-v3-Turbo via `ort` everywhere for correctness; consider Moonshine via `ort`/sherpa-onnx as the phone-tier fast path. Parakeet if English throughput dominates and you can carry FluidAudio (Apple) — but Parakeet has **no Rust/ort-native TDT decoder in-tree**; you'd use the community `parakeet-rs` (ONNX encoder/decoder + hand-written TDT loop) or sherpa-onnx.

### 4.2 Diarization ("who spoke when") [LEANING]
| Model | License | Notes |
|---|---|---|
| **pyannote/speaker-diarization-community-1** | **CC-BY-4.0** | The open baseline. ~15% DER real-world, exposes `exclusive_speaker_diarization` for clean STT-timestamp reconciliation. FluidAudio bundles it in CoreML (~122× RT). sherpa-onnx runs the pipeline cross-platform |
| pyannote 3.1 | MIT | legacy, pure PyTorch |
| **NVIDIA Streaming Sortformer** | **CC-BY-NC-4.0 ⚠️** | research-only; do NOT ship commercially |
| 3D-Speaker (CAM++/ERes2Net + clustering) | Apache 2.0 | sherpa-onnx-friendly, good multilingual |

**Two paradigms:** clustering pipeline (VAD → segmentation → embedding → AHC) for offline/batch; EEND (Sortformer/LS-EEND) for streaming. Both exposed by pyannote 4.0 and FluidAudio. **Default: clustering/offline for post-hoc tagging of recordings** (the primary use case).

### 4.3 Voiceprint identification ("who is this") [LEANING]
Reuses the diarizer's embedding model. Workflow = **k-NN over a small in-memory vector store of cosine-normalized embeddings.**

| Embedder | Dim | License | Note |
|---|---|---|---|
| **ECAPA-TDNN** | 192 | Apache 2.0 (SpeechBrain) | Best speed/accuracy balance; EER ~0.9–1.7%; default for sherpa-onnx/English |
| **WeSpeaker-ResNet34-LM** | 256 | Apache 2.0 | Default embedder behind pyannote community-1 |
| **TitaNet-Large** | 192 | NVIDIA | Used by whisper-diarization pipeline |
| **CAM++** | 192/256 | Apache 2.0 | Best for Mandarin/multilingual |

**Do not bikeshed the embedder** — sub-1% EER differences are imperceptible at a ~10-person enrollment set. Use whatever the diarization SDK ships. The dominant real-world error is **channel/mic mismatch**, not embedding quality.

**Enrollment:** 30–60 s clean speech → VAD → windowed embeddings → average + L2-normalize → one voiceprint vector. **ID:** diarize → per-cluster centroid → cosine-sim vs enrolled prints → threshold (start ECAPA @0.65, TitaNet @0.70, **calibrate on real data**) → known-person / `Unknown_N` / low-confidence.

**Store `embedder_version` + a Terrain edge to the source recording.** Voiceprints are NOT portable across embedder versions; you need the source audio to re-extract on upgrade. Biometric-data consent (GDPR/BIPA/TX/WA) applies even for single-user — make per-person delete cascade, encrypt at rest.

### 4.4 TTS [OPEN — not researched this session]
> **Flag:** the source conversation focused on ASR/diarization. TTS needs its own research pass. Below is the *likely* shape only.

Candidate local TTS, all ONNX-friendly (fits the `ort` strategy):
- **Piper** (Rhasspy) — ONNX, ubiquitous, fast, runs everywhere via its runtime or sherpa-onnx. **Likely pragmatic default.**
- **Kokoro** — high-quality, small; FluidAudio ships `KokoroTtsManager` (Apple), `kokoro-onnx` cross-platform.
- **Parler-TTS / MetaVoice-1B** — available in candle-transformers (Rust-native path).
- **Kyutai TTS / Moshi** — CC-BY-4.0, streaming/full-duplex; relevant if polaris grows into spoken dialog.

**[OPEN] Action:** run a dedicated TTS ADR. Provisionally assume **Piper or Kokoro via `ort`** to keep one backend. Verify licenses, voice quality, latency on target hardware, and streaming support before committing.

---

## 5. Per-platform deployment matrix

| Target | Candle (T1) | `ort` (T2) | Vendor-native (T3) |
|---|---|---|---|
| iPhone 16+ ANE | ❌ (Metal GPU only) | ⚠️ partial CoreML/ANE | ✅ WhisperKit/FluidAudio full ANE |
| Apple Silicon Mac | ✅ Metal | ✅ Metal + partial CoreML | ✅ ANE+GPU |
| Snapdragon 8 Elite NPU | ❌ CPU only | ✅ QNN/Hexagon | n/a (Apple-only) |
| Pixel 9/10 TPU | ❌ CPU only | ⚠️ NNAPI partial | n/a |
| Intel Core Ultra NPU | ❌ CPU only | ✅ OpenVINO EP | n/a |
| Android CPU | ✅ NEON | ✅ XNNPACK | n/a |
| Browser (WASM) | ✅ first-class | ⚠️ onnxruntime-web | ❌ |
| Linux/Win + CUDA | ✅ | ✅ CUDA EP | n/a |

Middle column is the target. Note the mobile story: Candle on Android = CPU-only (battery-hungry, ~1–3× RT); `ort` captures NPU wins that Candle structurally cannot.

---

## 6. Crate architecture [DECIDED — revised 2026-07-06]

> Supersedes the original `src-tauri/crates/polaris` single-crate layout.
> Resolved in a design session: the audio-ML stack splits into three workspace
> crates so model logistics, inference capabilities, and assistant
> orchestration never share a module. Directory names carry an `ai-` prefix;
> package names do not.

```
quartermaster   (supplies)     hf-hub fetch, cache, registry, ort sessions
      ↑
instruments     (tools)        asr / tts / embed submodules + shared audio
      ↑    ↑                   frontend; DTOs w/ specta derives live here
      |    └──────────────────────────────┐
cartograph      (integration)          polaris  (assistant flows — future)
      ↑                                    ↑
src-tauri / crates/server — existing command mirror; new commands only
```

- **`crates/ai-quartermaster` (package `quartermaster`) — supplies, zero model
  semantics.** Model registry (id → repo, revision, files, *license* — §9
  becomes machine-checkable: no `License` variant, no registration), `hf-hub`
  download with progress, per-platform cache paths, integrity, load/unload,
  and `ort` environment/session construction. Owns per-target EP
  feature-gating (`coreml` Apple, `qnn`/`xnnpack` Android, `openvino`/`cuda`
  desktop) and the Nix story (`ORT_LIB_LOCATION` via devenv rather than ort's
  build-time binary download). Knows *which files* a model needs, never *what
  it computes*.
- **`crates/ai-instruments` (package `instruments`) — capabilities.** One
  submodule per capability (`asr`, `tts`, `embed`, later `lm`), each owning
  its complete toolchain: preprocessing, forward passes against
  quartermaster-provided sessions, decode loops, postprocessing. The shared
  audio frontend (`audio`: capture, decode, resample, VAD, mel) lives here.
  Output types are plain-data DTOs with serde + specta derives — these are
  what cross the IPC boundary — but the crate exposes **no transport-facing
  API of its own**.
- **`crates/ai-polaris` (package `polaris`) — the assistant integration
  layer.** Voice input, intent interpretation, dialog state, action routing
  into cartograph. Consumes instruments. Deliberately near-empty until the
  assistant milestone; transcription does **not** route through it.
- **cartograph — the single seam for explicit tool use.** Meeting
  transcription, voiceprint enrollment, etc. are cartograph modules that
  drive instrument pipelines and write results into the domain (Beacon
  timelines, Terrain speaker edges, VoicePrint Readings). Transports keep
  depending on cartograph exactly as today; voice features add commands,
  never a new command architecture.

Key principles:
- **Explicit tool use vs. assistant pipeline is a stable taxonomy, not a
  phase.** Meeting transcription is user-invoked tooling through cartograph
  forever; polaris enters only when intent interpretation does.
- **Per-capability traits (`AsrEngine`, `TtsEngine`, …); no unifying
  `Instrument` trait.** The capabilities share no signature shape — the crate
  split is the generalization.
- **Backends are feature-gated behind those traits** (`ort` primary; Candle
  reserved for LoRA + WASM; vendor-native a future Tier-3 slot) — same seam
  philosophy as `RegistryStore`/`TauriRegistryStore` and the single
  `invoke.ts` IPC seam.
- **Dependency direction is one-way:** quartermaster → instruments →
  {cartograph, polaris}. Instruments never sees domain types; that ignorance
  is what keeps a future remote-inference deployment (quartermaster +
  instruments behind an API) possible.
- **Decoding loops (Whisper autoregressive, Parakeet TDT) live in host Rust
  in instruments regardless of backend** — ONNX/Candle only provide
  encoder/decoder forward passes.
- **App-store rules decide bundling:** stores prohibit downloading executable
  code, so `libonnxruntime` ships in every app bundle (~15–25 MB, desktop
  *and* mobile). Model weights are data — fetched via `hf-hub` on first use
  into the app-data dir, then fully offline (first-launch download, then
  air-gapped, like FluidAudio).

---

## 7. Waypoints domain integration [LEANING]

The audio features map onto the existing schema with **no new primitives** — this is the payoff of the Landmark/Chart/Terrain model.

**Voiceprint = Reading(s) on a Person Landmark:**
```
Person (Chart)
├── name: Text
├── voiceprints[]: VoicePrint         # composite Feature, repeatable per channel
│   ├── embedding: Float32[192|256]
│   ├── embedder_version: Text        # "ecapa-tdnn-v1", "wespeaker-resnet34-lm-v2"
│   ├── enrolled_at: DateTime
│   ├── channel_label: Text?          # "macbook-mic", "iphone-handheld"
│   └── source_recording: <Terrain edge → source-audio Mark>
```

**A recording = a Mark (or a `Recording` Chart Landmark):**
- Diarization timeline → **Beacon** activity-log samples (append-only time-series; the raw substrate).
- Speaker-attributed segments → **Terrain edges** (kind=Reference, label="speaker") from the recording to Person Landmarks, timestamps as edge attributes.
- Derived "who said what, summarized" views → **Expeditions** (stored-query Landmarks over the Beacon stream).

**Query payoff:** "everything Alice said across all recordings this year" = a Terrain edge walk + date filter. No special-purpose schema.

**Layer discipline reminder:** keep the raw diarization sample stream in Beacon; keep the legible per-speaker analysis in the Atlas/Expedition tier. Do not conflate them (per existing project principle).

**Provenance:** transcripts and speaker tags carry provenance like any write; if an agent produces or corrects them, attribute it (never destructively promote). Preserving the `source_recording` edge also gives free re-diarization / re-embedding when models upgrade.

---

## 8. Customization stack — the "POMDP → Palm DP" fix [LEANING]

Three composable layers, cheapest first. Which one works depends on the decoder paradigm.

1. **Prompt biasing (minutes).** Whisper `initial_prompt` (AED decoder conditions on it) or LLM-decoder prompt (Canary-Qwen/Granite). Auto-generate a glossary from the user's PKM (Zotero/notes/Landmarks) and inject. Fixes ~60–80% of jargon. Guard against hallucination: `condition_on_previous_text=False`, tighten `compression_ratio`/`logprob` thresholds, VAD pre-filter.
2. **Hotword/word boosting (hours).** For CTC/RNN-T/TDT (Parakeet): Riva word-boost or NeMo CTC-WS context biasing with multi-spelling (`POMDP_pomdp_p o m d p`); or shallow-fusion with a tiny KenLM trained on the user's research corpus. RNN-T needs internal-LM subtraction (HAT/LODR).
3. **LoRA personalization (half-day, 1–5 h of the user's voice).** Whisper-Large-v3 + PEFT LoRA (r=32, q_proj+v_proj) + INT8, single consumer GPU or M-series Mac. ~60 MB adapter, hot-swappable. Expect 10–25% relative WER reduction on the user's voice, negligible general degradation. **This is Candle's job** (`candle-lora`) or a Python/PEFT desktop step feeding an adapter the runtime loads. Compose with layer 1 — they multiply.

**Data thresholds:** <30 min → biasing only; 1–5 h → LoRA sweet spot; 10+ h → consider partial full-FT; >50 h → full-FT beats LoRA on the target speaker at the cost of generality.

---

## 9. Licensing traps [DECIDED — hard constraints]

Waypoints ships permissive (public-domain-leaning) and self-hostable. Allowed: **MIT, Apache-2.0, CC-BY-4.0.** Watch for:
- ⚠️ **Canary-1B (original 2024)** = CC-BY-NC. Every *later* Canary is CC-BY-4.0. Don't grab the old one.
- ⚠️ **NVIDIA Streaming Sortformer** = CC-BY-NC. Research only.
- ⚠️ **SeamlessM4T, Meta MMS, Cohere Transcribe** = CC-BY-NC. Research only.
- ⚠️ **Parakeet-Unified-EN-0.6B (Apr 2026)** = NVIDIA Open Model License (not CC-BY-4.0) — read terms.
- ❌ **All AssemblyAI, OpenAI gpt-4o-transcribe, ElevenLabs, Deepgram** = API-only. No on-device path. Out of scope for local-first by definition.
- ✅ Safe defaults: Whisper/Distil-Whisper/Moonshine (MIT), Parakeet-CTC/RNNT/TDT + Canary-Flash/v2/Qwen (CC-BY-4.0), Granite Speech (Apache-2.0), pyannote community-1 (CC-BY-4.0), ECAPA/WeSpeaker/CAM++ (Apache-2.0).

---

## 10. Open questions to resolve with the user
- ~~[OPEN] Stack~~ **[RESOLVED 2026-07-06]:** Tauri (§0).
- **[OPEN] TTS ADR:** model + backend (§4.4). Provisional: Piper/Kokoro via `ort`.
- **[OPEN] Primary ASR:** Whisper-Turbo (correctness, 99 langs) vs Parakeet (throughput, needs FluidAudio/parakeet-rs). Decide by profiling the author's actual audio (dictation vs interviews vs noisy).
- **[OPEN] Streaming vs batch** for live dictation vs post-hoc recording analysis — likely both, different models (streaming Parakeet/Nemotron for live, Whisper/Parakeet batch for recordings).
- **[OPEN] Sync interaction:** transcripts/voiceprints as ops in the HLC op-log — confirm voiceprint blobs fit the `AtomicOp` edge-centric model and LWW conflict resolution, and that biometric blobs are handled under the encrypted-at-rest requirement.
- **[OPEN] Turso engine migration timing:** `ort` model-cache and vector store are engine-agnostic, but confirm the voiceprint vector store lives outside the row-replicated sync path (or is deliberately synced).

## 11. First concrete actions for Claude Code

Done as of 2026-07-06: stack confirmed (§0); `quartermaster`/`instruments`/`polaris` scaffolded with traits, DTOs, and inter-crate deps (§6). Remaining, in order:

1. Wire a minimal batch ASR path end-to-end in `instruments::asr`: quartermaster `hub` download → `audio` frontend (decode, resample, mel) → encoder/decoder session → tokenizer decode → `Transcript`. This is the step where `ort`, `hf-hub`, `symphonia`, `hound` deps land — along with the devenv `ORT_LIB_LOCATION` story and CI packaging changes (`jniLibs/` on Android, static link on iOS).
2. Add the `cartograph → instruments` edge: a transcription module in cartograph that drives the pipeline and writes results into the domain; expose it as new commands in the existing `src-tauri`/`crates/server` mirror. No new command architecture.
3. Add pyannote community-1 diarization via `ort`/sherpa-onnx; wire `instruments::embed` enroll/identify against a cosine vector store.
4. Model the `VoicePrint` Feature + `Recording` Mark + speaker Terrain edges (§7); keep the raw timeline in Beacon.
5. Defer LoRA (Candle), TTS, streaming/live dictation, and any Tier-3 native sidecar until the batch T2 path is proven end-to-end.
