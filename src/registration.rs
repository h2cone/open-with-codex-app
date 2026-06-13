#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistrationState {
    pub codex_app_installed: bool,
    pub context_menu_registered: bool,
    pub cli_registered: bool,
    pub cli_path_registered: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrationAction {
    SkipMissingCodexApp,
    RegisterContextMenu,
    InstallCliShim,
    RegisterCliPath,
}

pub fn plan_registration(state: &RegistrationState) -> Vec<RegistrationAction> {
    if !state.codex_app_installed {
        return vec![RegistrationAction::SkipMissingCodexApp];
    }

    let mut actions = Vec::new();
    if !state.context_menu_registered {
        actions.push(RegistrationAction::RegisterContextMenu);
    }
    if !state.cli_registered {
        actions.push(RegistrationAction::InstallCliShim);
    }
    if !state.cli_path_registered {
        actions.push(RegistrationAction::RegisterCliPath);
    }
    actions
}
