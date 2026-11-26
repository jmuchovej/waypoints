// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

use bedrock::Scalar;

// A data-carrying variant is not a scalar; it belongs in a Document.
#[derive(Scalar)]
enum HasData {
    Unit,
    WithData(u32),
}

fn main() {}
