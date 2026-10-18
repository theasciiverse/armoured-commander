//! Windows not ported yet, and the campaign day screen's drawing, which
//! the encounter code calls. Each says so when opened.

use crate::game::*;

impl Game {
    fn not_ported(&mut self, what: &str) {
        self.popup(&format!("{what} is not ported yet."));
    }

    /// `ShowHelp`
    pub fn show_help(&mut self) {
        self.not_ported("Help");
    }

    /// `ShowTankInfo`
    pub fn show_tank_info(&mut self, _select_tank: bool) {
        self.not_ported("Tank Info");
    }

    /// `ShowCrewInfo`
    pub fn show_crew_info(&mut self) {
        self.not_ported("Crew Info");
    }

    /// `ShowSettings`
    pub fn show_settings(&mut self) {
        self.not_ported("Settings");
    }

    /// `ShowCampaignStats`
    pub fn show_campaign_stats(&mut self) {
        self.not_ported("Campaign Stats");
    }

    /// `MainGunAmmoMenu`
    pub fn main_gun_ammo_menu(&mut self, _no_dark: bool) {
        self.not_ported("The main gun ammunition menu");
    }

    /// `RenderCampaign`
    pub fn render_campaign(&mut self, _no_flush: bool) {}

    /// `UpdateCActionCon`
    pub fn update_c_action_con(&mut self) {}

    /// `UpdateCInfoCon`
    pub fn update_c_info_con(&mut self, _mx: i32, _my: i32) {}

    /// `UpdateCOverlay`
    pub fn update_c_overlay(&mut self, _highlight_node: Option<usize>) {}
}
