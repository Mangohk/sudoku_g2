//! Technique catalog for the later skill solver.
//!
//! These ids are the stable names the trace will store. The array order is the
//! order the solver will try. Nothing here grades a generated puzzle.

/// Where a technique sits in the guide: the playable catalog, or the appendix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TechniqueFamily {
    Game,
    Appendix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Technique {
    pub id: &'static str,
    pub name: &'static str,
    pub family: TechniqueFamily,
}

const fn game(id: &'static str, name: &'static str) -> Technique {
    Technique {
        id,
        name,
        family: TechniqueFamily::Game,
    }
}

const fn appendix(id: &'static str, name: &'static str) -> Technique {
    Technique {
        id,
        name,
        family: TechniqueFamily::Appendix,
    }
}

/// The 32 playable skills, in the order the solver will try them.
pub const GAME: &[Technique] = &[
    game("full_house", "Full House"),
    game("naked_single", "Naked Single"),
    game("hidden_single", "Hidden Single"),
    game("pointing", "Locked Candidates Type 1 (Pointing)"),
    game("claiming", "Locked Candidates Type 2 (Claiming)"),
    game("naked_pair", "Naked Pair"),
    game("hidden_pair", "Hidden Pair"),
    game("naked_triple", "Naked Triple"),
    game("hidden_triple", "Hidden Triple"),
    game("xy_wing", "XY-Wing (Y-Wing)"),
    game("xyz_wing", "XYZ-Wing"),
    game("naked_quad", "Naked Quad"),
    game("hidden_quad", "Hidden Quad"),
    game("x_wing", "X-Wing"),
    game("w_wing", "W-Wing"),
    game("skyscraper", "Skyscraper"),
    game("two_string_kite", "2-String Kite"),
    game("turbot_fish", "Turbot Fish"),
    game("empty_rectangle", "Empty Rectangle"),
    game("wxyz_wing", "WXYZ-Wing"),
    game("finned_x_wing", "Finned and Sashimi X-Wing"),
    game("swordfish", "Swordfish"),
    game("remote_pairs", "Remote Pairs"),
    game("unique_rectangle", "Unique Rectangle (types 1 to 6)"),
    game("finned_swordfish", "Finned and Sashimi Swordfish"),
    game("jellyfish", "Jellyfish"),
    game(
        "hidden_avoidable_rectangle",
        "Hidden and Avoidable Rectangles",
    ),
    game("bug_plus_one", "BUG+1"),
    game("simple_colors", "Simple Colors"),
    game("multi_colors", "Multi-Colors"),
    game("x_chain", "X-Chain"),
    game("xy_chain", "XY-Chain"),
];

/// The 15 appendix methods, including the DLX / CSP backend.
pub const APPENDIX: &[Technique] = &[
    appendix("franken_mutant_fish", "Franken and Mutant Fish"),
    appendix("kraken_fish", "Kraken Fish"),
    appendix("medusa_3d", "3D Medusa"),
    appendix("nice_loops", "Nice Loops"),
    appendix("aic", "AIC"),
    appendix("sue_de_coq", "Sue de Coq"),
    appendix("als", "ALS, including ALS-XY-Wing and ALS-Chain"),
    appendix("death_blossom", "Death Blossom"),
    appendix("ape_ate", "APE / ATE"),
    appendix("fireworks", "Fireworks"),
    appendix("exocet", "Exocet, junior and double"),
    appendix("tridagon", "Gurth / Tridagon"),
    appendix("forcing_chains_nets", "Forcing Chains and Forcing Nets"),
    appendix(
        "nishio_templates_bowman",
        "Nishio, Templates, and Bowman Bingo",
    ),
    appendix("dlx_csp", "DLX / CSP solver backend"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guide_catalog_is_32_game_skills_and_15_appendix_methods() {
        assert_eq!(GAME.len(), 32);
        assert_eq!(APPENDIX.len(), 15);
        assert_eq!(GAME[0].id, "full_house");
        assert_eq!(GAME[31].id, "xy_chain");
        assert_eq!(APPENDIX[14].id, "dlx_csp");
        assert!(GAME.iter().all(|tech| tech.family == TechniqueFamily::Game));
        assert!(APPENDIX
            .iter()
            .all(|tech| tech.family == TechniqueFamily::Appendix));

        let mut ids: Vec<&str> = GAME.iter().chain(APPENDIX).map(|tech| tech.id).collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), before);
    }
}
