use bedrock::{Collection, Scalar};
use uuid::Uuid;

#[derive(Scalar)]
struct Id(Uuid);

// A discriminant scopes into a document's JSON, so it must be a document column,
// not a scalar — rejected at compile time.
#[derive(Collection)]
struct Bad {
    #[bedrock(id)]
    id: Id,
    #[bedrock(discriminant)]
    kind: String,
}

fn main() {}
