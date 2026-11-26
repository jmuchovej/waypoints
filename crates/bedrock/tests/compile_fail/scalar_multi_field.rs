// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

use bedrock::Scalar;

// Scalar is for newtypes (one field), not multi-field tuple structs.
#[derive(Scalar)]
struct TwoFields(u32, u32);

fn main() {}
