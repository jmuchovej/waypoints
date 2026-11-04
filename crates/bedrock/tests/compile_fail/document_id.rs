// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

use bedrock::{Collection, Document};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Document)]
struct Payload {
    x: i32,
}

// The id is a document, not a scalar column — rejected at compile time.
#[derive(Collection)]
struct DocumentId {
    #[bedrock(id)]
    id: Payload,
    name: String,
}

fn main() {}
