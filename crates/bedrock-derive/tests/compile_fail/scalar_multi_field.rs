use bedrock_derive::Scalar;

// Scalar is for newtypes (one field), not multi-field tuple structs.
#[derive(Scalar)]
struct TwoFields(u32, u32);

fn main() {}
