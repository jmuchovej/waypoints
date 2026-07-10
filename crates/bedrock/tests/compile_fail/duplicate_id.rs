use bedrock::{Collection, Scalar};
use uuid::Uuid;

#[derive(Scalar)]
struct Id(Uuid);

// Two `#[bedrock(id)]` markers — a collection has exactly one id.
#[derive(Collection)]
struct TwoIds {
    #[bedrock(id)]
    a: Id,
    #[bedrock(id)]
    b: Id,
}

fn main() {}
