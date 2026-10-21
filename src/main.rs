mod app;
mod battle_data;
mod battle_time;
mod battle_rng;
mod battle_ai;
mod battle_scheduler;
mod battle_commands;
mod battle_advance;
mod battle_action_init;
mod characters;
mod equipment;
mod damage;
mod elemental_damage;
mod damage_application;
mod battle_accuracy;
mod battle_hit_gates;
mod battle_hit;
mod battle_equipment;
mod battle_actor_stats;
mod battle_rage;
mod battle_targets;
mod battle_target_init;
mod battle_resources;
mod battle_attack_prep;
mod battle_status_requests;
mod battle_status_transition;
mod battle_status_update;
mod battle_cleanup;
mod battle_readiness;
mod battle_menu_state;
mod battle_after_effects;
mod battle_morph;
mod battle_after_action;
mod battle_special_status;
mod battle_presence;
mod battle_retaliation;
mod battle_damage_exec;
mod battle_target_damage;
mod battle_attack_history;
mod battle_hit_resolution;
mod battle_target_effects;
mod battle_inventory;
mod battle_command_init;
mod battle_spell_validation;
mod battle_spell_lists;
mod battle_party;
mod battle_character_load;
mod battle_monster_load;
mod battle_entry;
mod battle_action_damage;
mod battle_attack_effect;
mod battle_cast;
mod battle_reflect;
mod battle_counted;
mod battle_item_effect;
mod assets;
mod magitek;
mod player_motion;
mod field;
mod field_events;
mod field_exit;
mod event_vm;
mod dialogue_text;
mod gfx;

#[cfg(windows)]
mod platform_windows;

fn main() {
    let mut args = std::env::args_os().skip(1);
    let action = args.next();
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-item-effect")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-item-effect"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_item_effect::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-counted")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-counted"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_counted::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-reflect")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-reflect"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_reflect::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-cast")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-cast"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_cast::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-attack-effect")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-attack-effect"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_attack_effect::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-action-damage")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-action-damage"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_action_damage::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-entry")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-entry"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_entry::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-monster-load")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-monster-load"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_monster_load::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-character-load")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-character-load"));
        if let Err(error)=battle_character_load::capture(&directory) {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-party")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-party"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_party::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-spell-lists")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-spell-lists"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_spell_lists::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-spell-validation")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-spell-validation"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_spell_validation::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-command-init")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-command-init"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_command_init::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-field-inventory")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-field-inventory"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_inventory::capture_field(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-inventory")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-inventory"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_inventory::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-target-effects")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-target-effects"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_target_effects::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-hit-resolution")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-hit-resolution"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_hit_resolution::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-attack-history")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-attack-history"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_attack_history::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-target-damage")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-target-damage"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_target_damage::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-damage-exec")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-damage-exec"));
        if let Err(error)=battle_damage_exec::capture(&directory) {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-retaliation")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-retaliation"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_retaliation::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-presence")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-presence"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_presence::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-special-status")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-special-status"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_special_status::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-after-action")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-after-action"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_after_action::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-morph")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-morph"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_morph::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-after-effects")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-after-effects"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_after_effects::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-menu-state")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-menu-state"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_menu_state::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-readiness")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-readiness"));
        if let Err(error)=battle_readiness::capture(&directory) {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-cleanup")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-cleanup"));
        if let Err(error)=battle_cleanup::capture(&directory) {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-status-update")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-status-update"));
        if let Err(error)=battle_status_update::capture(&directory) {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-status-transition")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-status-transition"));
        if let Err(error)=battle_status_transition::capture(&directory) {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-status-requests")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-status-requests"));
        if let Err(error)=battle_status_requests::capture(&directory) {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-attack-prep")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-attack-prep"));
        if let Err(error)=battle_attack_prep::capture(&directory) {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-resources")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-resources"));
        if let Err(error)=battle_resources::capture(&directory) {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-target-init")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-target-init"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_target_init::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-targets")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-targets"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_targets::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-ai-turn")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-ai-turn"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_ai::capture_turn(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-ai-init")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-ai-init"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_ai::capture_init(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-rage")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-rage"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_rage::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-actor-stats")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-actor-stats"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_actor_stats::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-equipment")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-equipment"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_equipment::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-hit")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-hit"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_hit::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-hit-gates")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-hit-gates"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_hit_gates::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-accuracy")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-accuracy"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_accuracy::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-damage-application")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-damage-application"));
        if let Err(error)=damage_application::capture(&directory) {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-elemental-damage")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-elemental-damage"));
        if let Err(error)=elemental_damage::capture(&directory) {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-action-init")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-action-init"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_action_init::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-advance")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-advance"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_advance::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-commands")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-commands"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_commands::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-scheduler")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-scheduler"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_scheduler::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-battle-rng")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-rng"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|battle_rng::capture(root,&directory).and_then(|_|battle_ai::capture(root,&directory)));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-damage")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-damage"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string()).and_then(|root|damage::capture(root,&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref()==Some(std::ffi::OsStr::new("--capture-equipment")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-equipment"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string())
            .and_then(|root|equipment::ItemTable::load(root).and_then(|table|table.capture(root,&directory)))
            .and_then(|_|app::capture_equipment(&directory).map_err(|e|e.to_string()));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}return;
    }
    if action.as_deref() == Some(std::ffi::OsStr::new("--capture-characters")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-characters"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string())
            .and_then(|root|characters::CharacterTables::load(root)).and_then(|data|data.capture(&directory))
            .and_then(|_|app::capture_characters(&directory).map_err(|e|e.to_string()));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}
        println!("Character data captured to {}",directory.display());return;
    }
    if action.as_deref() == Some(std::ffi::OsStr::new("--capture-battle-data")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-battle-data"));
        let assets=assets::Assets::load();
        let result=assets.root.as_ref().ok_or_else(||"Asset root unavailable".to_string())
            .and_then(|root|battle_data::BattleData::load(root)).and_then(|data|data.capture(&directory))
            .and_then(|_|battle_time::capture(&directory));
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}
        println!("Battle tables captured to {}",directory.display());return;
    }
    if action.as_deref() == Some(std::ffi::OsStr::new("--inspect-first-battle")) {
        let assets=assets::Assets::load();
        let result=(|| -> Result<(),String> {
            let root=assets.root.as_ref().ok_or("Asset root unavailable")?;
            let data=battle_data::BattleData::load(root)?;
            let id=data.event_formation(1,0)?;
            if id!=data.event_formation(1,255)? {return Err("Encounter depends on RNG".into());}
            let encounter=data.encounter(id)?;
            println!("FORMATION {} PROPERTIES {:02X?}",id,encounter.loaded_properties);
            for m in encounter.monsters {println!("SLOT {} MONSTER {} PRESENT {} HP {} MP {} EXP {} GIL {}",m.slot,m.monster_id,m.present,m.hp,m.mp,m.properties.exp(),m.properties.gil());}
            Ok(())
        })();
        if let Err(error)=result {eprintln!("{error}");std::process::exit(1);}
        return;
    }
    if action.as_deref() == Some(std::ffi::OsStr::new("--trace-field-events")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-field-events"));
        let assets=assets::Assets::load();
        if let Err(error)=event_vm::capture_field_event_traces(&assets.event_script,&assets.field_events,&directory){eprintln!("Field event trace failed: {error}");std::process::exit(1);}
        println!("Field event traces saved to {}",directory.display());return;
    }
    if action.as_deref() == Some(std::ffi::OsStr::new("--capture-trigger-scenes")) {
        let directory=args.next().map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("reverse-engineering/captures/native-trigger-scenes"));
        if let Err(error)=app::capture_trigger_scenes(&directory){eprintln!("Trigger scenes failed: {error}");std::process::exit(1);}
        println!("Triggered scenes saved to {}",directory.display());return;
    }
    if action.as_deref() == Some(std::ffi::OsStr::new("--capture-player-motion")) {
        let directory = args.next().map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("reverse-engineering/captures/native-player-motion"));
        if let Err(error) = app::capture_player_motion(&directory) {
            eprintln!("Player motion capture failed: {error}");std::process::exit(1);
        }
        println!("Player motion saved to {}",directory.display());return;
    }
    if action.as_deref() == Some(std::ffi::OsStr::new("--capture-magitek")) {
        let directory = args.next().map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("reverse-engineering/captures/native-magitek"));
        if let Err(error) = magitek::capture_atlas(&directory) {
            eprintln!("Magitek atlas failed: {error}");
            std::process::exit(1);
        }
        println!("Magitek atlas saved to {}", directory.display());
        return;
    }
    if action.as_deref() == Some(std::ffi::OsStr::new("--capture-opening-motion")) {
        let directory = args.next().map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("reverse-engineering/captures/native-opening-motion"));
        if let Err(error) = app::capture_opening_motion(&directory) {
            eprintln!("Opening motion capture failed: {error}");
            std::process::exit(1);
        }
        println!("Live opening motion saved to {}", directory.display());
        return;
    }
    if action.as_deref() == Some(std::ffi::OsStr::new("--trace-opening-timeline")) {
        let directory = args.next().map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("reverse-engineering/captures/native-opening-timeline"));
        let assets = assets::Assets::load();
        if let Err(error) = event_vm::capture_opening_timeline(&assets.event_script, &directory) {
            eprintln!("Event timeline failed: {error}");
            std::process::exit(1);
        }
        println!("Scheduled opening timeline saved to {}", directory.display());
        return;
    }
    if action.as_deref() == Some(std::ffi::OsStr::new("--trace-opening-event")) {
        let directory = args.next().map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("reverse-engineering/captures/native-opening-event"));
        let assets = assets::Assets::load();
        if let Err(error) = event_vm::capture_opening_entry(&assets.event_script, &directory) {
            eprintln!("Event trace failed: {error}");
            std::process::exit(1);
        }
        println!("Opening event trace saved to {}", directory.display());
        return;
    }
    if action.as_deref() == Some(std::ffi::OsStr::new("--capture-field-route")) {
        let directory = args.next().map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("reverse-engineering/captures/native-field-route"));
        if let Err(error) = app::capture_field_route(&directory) {
            eprintln!("Field traversal capture failed: {error}");
            std::process::exit(1);
        }
        println!("Field traversal frames and state trace saved to {}", directory.display());
        return;
    }
    if action.as_deref() == Some(std::ffi::OsStr::new("--capture-opening")) {
        let directory = args.next().map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("reverse-engineering/captures/native-opening"));
        if let Err(error) = app::capture_opening(&directory) {
            eprintln!("Opening capture failed: {error}");
            std::process::exit(1);
        }
        println!("Opening frames and state trace saved to {}", directory.display());
        return;
    }
    #[cfg(windows)]
    if action.as_deref() == Some(std::ffi::OsStr::new("--smoke-window")) {
        if let Err(error) = platform_windows::smoke_test() {
            eprintln!("Win32/GDI smoke test failed: {error}");
            std::process::exit(1);
        }
        println!("Win32/GDI startup rendered three frames and closed normally.");
        return;
    }
    #[cfg(windows)]
    if let Err(error) = platform_windows::run() {
        eprintln!("FF6 Native could not start: {error}");
        std::process::exit(1);
    }
    #[cfg(not(windows))]
    {
    eprintln!("FF6 Native's first renderer currently uses the Windows GDI backend.");
    eprintln!("The game state and software renderer are platform independent.");
    }
}
