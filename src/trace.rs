//! Solving-trace types stored for a later replay UI.
//!
//! Phase 1 does not produce these steps. Intake already accepts them, and it
//! rejects a skill step whose reason is missing a part. Candidate masks use one
//! byte per cell: bit 0 is digit 1, bit 8 is digit 9.

use serde::{Deserialize, Serialize};

/// Ordered steps that are enough to replay a solve without running it again.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Trace {
    pub catalog_rev: String,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Step {
    pub index: u32,
    pub skill_id: String,
    pub template_id: String,
    pub facts: serde_json::Value,
    pub pattern: Vec<PatternCell>,
    pub placements: Vec<Placement>,
    pub eliminations: Vec<Placement>,
    pub reason: Reason,
    /// 81 bytes. Bit `(digit - 1)` is set when that digit is still a candidate.
    pub candidates_after: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatternCell {
    pub r: u8,
    pub c: u8,
    pub role: String,
    pub notes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Placement {
    pub r: u8,
    pub c: u8,
    pub d: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reason {
    pub short: String,
    pub detailed: String,
    pub observation: String,
    pub deduction: String,
    pub action: String,
}
