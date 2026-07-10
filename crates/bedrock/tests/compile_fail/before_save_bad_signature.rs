use bedrock::{Collection, Scalar};
use uuid::Uuid;

#[derive(Scalar)]
struct Id(Uuid);

// Wrong signature: missing the `WriteOp` parameter. The derive binds the named
// function to `fn(Self, WriteOp) -> Self`, so this fails to compile.
fn wrong(doc: Bad) -> Bad {
    doc
}

#[derive(Collection)]
#[bedrock(before_save = wrong)]
struct Bad {
    #[bedrock(id)]
    id: Id,
    n: i64,
}

fn main() {}
