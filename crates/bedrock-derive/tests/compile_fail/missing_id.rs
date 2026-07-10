use bedrock_derive::Collection;

// No field is marked `#[bedrock(id)]` — there is no implicit id.
#[derive(Collection)]
struct NoId {
    name: String,
    count: i64,
}

fn main() {}
