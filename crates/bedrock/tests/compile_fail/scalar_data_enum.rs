use bedrock::Scalar;

// A data-carrying variant is not a scalar; it belongs in a Document.
#[derive(Scalar)]
enum HasData {
    Unit,
    WithData(u32),
}

fn main() {}
