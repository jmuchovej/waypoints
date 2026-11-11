// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

use bedrock::Collection;

// No field is marked `#[bedrock(id)]` — there is no implicit id.
#[derive(Collection)]
struct NoId {
    name: String,
    count: i64,
}

fn main() {}
