use std::fs;
use std::time::Duration;

use crate::assets::{Assets, SpriteSheet};
use crate::event_vm::{EventState, EventVm, EventYield, NARSHE_ENTRY, SWITCH_BYTES};
use crate::gfx::{Canvas, BLACK, GOLD, PALE_BLUE, RED, WHITE, WIDTH};

#[derive(Clone, Copy, Default)]
pub struct Input {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub confirm: bool,
    pub cancel: bool,
    pub menu: bool,
    pub dash: bool,
    pub save: bool,
    pub load: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scene {
    Title,
    Intro,
    Field,
    Menu,
    Battle,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MenuPage {
    Main,
    Items,
    Magic,
    Equipment,
    Status,
    Save,
    Load,
    Config,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DialogueCompletion {
    ResumeField,
    EnterNarshe,
    ResumeEvent,
}

struct Dialogue {
    lines: Vec<String>,
    current: usize,
    completion: DialogueCompletion,
}

struct Battle {
    cursor: usize,
    enemy_hp: i32,
    enemy_max_hp: i32,
    message: Option<String>,
    victory: bool,
    escaped: bool,
}

pub struct Game {
    scene: Scene,
    menu_page: MenuPage,
    menu_cursor: usize,
    intro_index: usize,
    opening_phase: u8,
    dialogue: Option<Dialogue>,
    assets: Assets,
    characters: crate::characters::Roster,
    character_tables: crate::characters::CharacterTables,
    item_table: crate::equipment::ItemTable,
    event_state: EventState,
    event_vm: Option<EventVm>,
    event_clock: f64,
    event_fault: bool,
    event_dialogue: Option<(u16,u16)>,
    story_events: bool,
    npc_map: Option<u16>,
    map_id: u16,
    x: i16,
    y: i16,
    facing: usize,
    hp: i32,
    max_hp: i32,
    mp: i32,
    max_mp: i32,
    potions: u8,
    gil: u32,
    exp: u32,
    level: u8,
    steps: u32,
    motion: crate::player_motion::PlayerMotion,
    field_clock: f64,
    field_frame: u64,
    pending_exit: bool,
    pending_menu: bool,
    elapsed: f32,
    rng: u32,
    battle: Option<Battle>,
    last_action: String,
}

const INTRO: [[&str; 4]; 4] = [
    [
        "LONG AGO, THE WAR OF THE MAGI",
        "REDUCED THE WORLD TO A",
        "SCORCHED WASTELAND, AND MAGIC",
        "SIMPLY CEASED TO EXIST.",
    ],
    [
        "A THOUSAND YEARS HAVE PASSED...",
        "IRON, GUNPOWDER, AND STEAM",
        "ENGINES HAVE BEEN DEVELOPED,",
        "AND HIGH TECHNOLOGY REIGNS...",
    ],
    [
        "BUT NOW THERE ARE SOME WHO",
        "WOULD ENSLAVE THE WORLD BY",
        "REVIVING THE DREAD DESTRUCTIVE",
        "FORCE KNOWN AS MAGIC.",
    ],
    [
        "CAN IT BE THAT THOSE",
        "IN POWER ARE ON THE VERGE OF",
        "REPEATING A SENSELESS",
        "AND DEADLY MISTAKE?",
    ],
];

// The opening event's DLG_6–DLG_9 cards load these maps and coordinates in order.
const INTRO_MAPS: [(u16, i16, i16); 4] = [(23, 18, 25), (20, 38, 54), (244, 7, 27), (18, 11, 7)];

const OPENING_DIALOGUE: [&str; 7] = [
    "BIGGS: THERE'S THE CITY...",
    "WEDGE: HARD TO BELIEVE AN ESPER'S BEEN FOUND INTACT THERE, A THOUSAND YEARS AFTER THE WAR OF THE MAGI...",
    "BIGGS: THINK IT'S STILL ALIVE?",
    "WEDGE: PROBABLY... JUDGING FROM THE URGENCY OF OUR ORDERS. PLUS, THEY HAD US BRING THIS SORCERESS WITH US.",
    "BIGGS: YEAH... OUR WITCH. WHY'S SHE HERE? I HEARD SHE FRIED FIFTY OF OUR MAGITEK ARMORED SOLDIERS IN UNDER THREE MINUTES. KIND OF MAKES YOUR SKIN CRAWL...",
    "WEDGE: RELAX. THE SLAVE CROWN ON HER HEAD MAKES HER A MINDLESS PUPPET. SHE'LL FOLLOW ORDERS.",
    "WEDGE: WE'LL APPROACH FROM THE EAST. MOVE OUT!",
];

const MENU_ITEMS: [&str; 8] = [
    "ITEMS",
    "MAGIC",
    "EQUIPMENT",
    "STATUS",
    "SAVE",
    "LOAD",
    "CONFIG",
    "RETURN",
];

const BATTLE_COMMANDS: [&str; 4] = ["FIGHT", "MAGIC", "ITEM", "RUN"];

// Tile offsets from upstream src/field/sprite_data.asm MapSpriteTileOffsets,
// converted from byte offsets to 32-byte 4bpp tile indices.
const WALK_DOWN: [[usize; 6]; 3] = [
    [0, 1, 2, 3, 4, 5],
    [0, 1, 6, 7, 8, 9],
    [0, 1, 10, 11, 4, 5],
];
const WALK_UP: [[usize; 6]; 3] = [
    [12, 13, 14, 15, 16, 17],
    [12, 13, 18, 19, 20, 21],
    [12, 13, 22, 23, 16, 17],
];
const WALK_LEFT: [[usize; 6]; 3] = [
    [24, 25, 26, 27, 28, 29],
    [30, 31, 32, 33, 34, 35],
    [24, 25, 36, 37, 38, 39],
];

impl Game {
    pub fn new() -> Self {
        let assets = Assets::load();
        let character_tables=assets.root.as_ref().ok_or("Asset root unavailable")
            .and_then(|root|crate::characters::CharacterTables::load(root).map_err(|_|"Character tables unavailable: run tools/extract_native_characters.py"))
            .expect("Verified character runtime tables are required");
        let characters=character_tables.opening_party().expect("Opening character initialization failed");
        let item_table=crate::equipment::ItemTable::load(assets.root.as_ref().unwrap()).expect("Verified target item tables are required");
        let terra=characters[0].as_ref().unwrap();
        let (hp,mp,exp,level)=(i32::from(terra.hp),i32::from(terra.mp),terra.exp,terra.level);
        let mut event_state=EventState::default();assets.field_events.initialize_switches(&mut event_state);
        event_state.set_switch(0x2e0,true);event_state.set_switch(0x2f0,true);
        Self {
            scene: Scene::Title,
            menu_page: MenuPage::Main,
            menu_cursor: 0,
            intro_index: 0,
            opening_phase: 0,
            dialogue: None,
            assets,
            characters,
            character_tables,
            item_table,
            event_state,
            event_vm: None,
            event_clock: 0.0,
            event_fault: false,
            event_dialogue: None,
            story_events: true,
            npc_map: None,
            map_id: 19,
            x: 38,
            y: 58,
            facing: 0,
            hp,
            max_hp: hp,
            mp,
            max_mp: mp,
            potions: 3,
            gil: 0,
            exp,
            level,
            steps: 0,
            motion: crate::player_motion::PlayerMotion::default(),
            field_clock: 0.0,
            field_frame: 0,
            pending_exit: false,
            pending_menu: false,
            elapsed: 0.0,
            rng: 0x6f66_6636,
            battle: None,
            last_action: String::from("THE SNOW IS DEEP. FOLLOW THE TRACK."),
        }
    }

    pub fn update(&mut self, input: Input, delta: Duration) {
        let dt = delta.as_secs_f32().min(0.1);
        self.elapsed += dt;

        if input.save && self.scene != Scene::Title && self.scene != Scene::Intro {
            let result = self.save_state();
            if result.is_ok() {
                self.last_action = String::from("GAME SAVED TO SLOT 1.");
            }
        }
        if input.load && self.scene != Scene::Title && self.scene != Scene::Intro {
            if self.load_state().is_ok() {
                return;
            }
        }

        if self.event_vm.is_some() && self.dialogue.is_none() {
            if self.event_fault { return; }
            self.event_clock += f64::from(dt) * 60.0;
            while self.event_clock >= 1.0 && self.event_vm.is_some() && self.dialogue.is_none() {
                self.event_clock -= 1.0;
                self.advance_event();
            }
            return;
        }

        if self.dialogue.is_some() {
            if input.confirm {
                let has_more = self
                    .dialogue
                    .as_ref()
                    .is_some_and(|dialogue| dialogue.current + 1 < dialogue.lines.len());
                if has_more {
                    if let Some(dialogue) = &mut self.dialogue {
                        dialogue.current += 1;
                    }
                } else {
                    let completion = self.dialogue.take().unwrap().completion;
                    self.complete_dialogue(completion);
                }
            }
            return;
        }

        match self.scene {
            Scene::Title => {
                if input.confirm {
                    self.intro_index = 0;
                    self.scene = Scene::Intro;
                    self.event_state.set_switch(0x1cc, true);
                }
            }
            Scene::Intro => {
                if input.cancel {
                    self.scene = Scene::Title;
                } else if input.confirm {
                    self.intro_index += 1;
                    if self.intro_index >= INTRO.len() {
                        self.scene = Scene::Field;
                        self.opening_phase = 1;
                        self.map_id = 18;
                        self.x = 11;
                        self.y = 7;
                        self.script_dialogue(&OPENING_DIALOGUE, DialogueCompletion::EnterNarshe);
                    }
                }
            }
            Scene::Field => {
                self.pending_menu |= input.menu;
                self.field_clock += f64::from(dt) * 60.0;
                while self.field_clock >= 1.0 && self.scene == Scene::Field && self.dialogue.is_none() && self.event_vm.is_none() {
                    self.field_clock -= 1.0;
                    self.update_field(input);
                }
            }
            Scene::Menu => self.update_menu(input),
            Scene::Battle => self.update_battle(input),
        }
    }

    pub fn draw(&self, canvas: &mut Canvas) {
        match self.scene {
            Scene::Title => self.draw_title(canvas),
            Scene::Intro => self.draw_intro(canvas),
            Scene::Field => self.draw_field(canvas),
            Scene::Menu => {
                self.draw_field(canvas);
                self.draw_menu(canvas);
            }
            Scene::Battle => self.draw_battle(canvas),
        }
    }

    fn player_world_pixel(&self) -> (i32,i32) {
        let (dx,dy) = self.motion.offset();
        (i32::from(self.x)*16+dx/256, i32::from(self.y)*16+dy/256)
    }

    fn update_field(&mut self, input: Input) {
        if self.event_vm.is_some() { return; }
        self.field_frame = self.field_frame.wrapping_add(1);
        if self.story_events && !self.motion.active {
            if let Some(entry)=self.assets.field_events.trigger_at(self.map_id,self.x,self.y) {
                self.start_field_event(entry);
                if self.event_vm.is_some() || self.dialogue.is_some() {return;}
            }
        }
        // FieldLoop checks entrances before object commands/position updates.
        if self.pending_exit && !self.motion.active {
            self.pending_exit = false;
            let previous_map = self.map_id;
            self.check_entrance();
            if self.map_id != previous_map { return; }
        }
        if self.pending_menu && !self.motion.active {
            self.pending_menu = false;
            self.field_clock = 0.0;
            self.menu_page = MenuPage::Main;
            self.menu_cursor = 0;
            self.scene = Scene::Menu;
            return;
        }
        if !self.motion.active && self.field_frame % 4 == 0 {
            // player.asm gives right/left precedence over up/down.
            let direction = if input.right { Some((1,0,1)) }
                else if input.left { Some((-1,0,3)) }
                else if input.up { Some((0,-1,0)) }
                else if input.down { Some((0,1,2)) } else { None };
            if let Some((dx,dy,facing)) = direction {
                self.facing = facing;
                if self.walkable(self.x+dx,self.y+dy,dx,dy) {
                    let speed = crate::player_motion::patched_speed(input.dash,self.party_field_effects()&0x20!=0);
                    self.motion.start(facing as u8,speed);
                    self.steps += 1;
                }
            }
        }
        if let Some((dx,dy)) = self.motion.advance() {
            self.x += dx;
            self.y += dy;
            self.pending_exit = true;
        }
    }

    fn party_field_effects(&self)->u8 {
        let mut effects=0;
        for (slot,character) in self.characters.iter().enumerate() {
            if self.event_state.actors[slot].enabled && self.event_state.char_parties[slot]==self.event_state.active_party {
                if let Some(character)=character {
                    if let Ok(stats)=self.item_table.calculate(character,character.equipment(),0) {effects|=stats.field_effects();}
                }
            }
        }
        effects
    }

    fn check_entrance(&mut self) {
        let Some(exit) = self.assets.field_maps.get(&self.map_id)
            .and_then(|map| map.exit_at(self.x, self.y)) else { return };
        let destination = exit.destination_map();
        // World-map and parent-map state require their dedicated subsystems.
        // Preserve their records but do not interpret them as ordinary maps.
        if destination < 3 || destination == 0x01ff || exit.map_word & 0x0200 != 0 {
            return;
        }
        let Some(map) = self.assets.field_maps.get(&destination) else { return };
        if !map.in_bounds(exit.destination_x, exit.destination_y) {
            return;
        }
        self.map_id = destination;
        self.x = exit.destination_x;
        self.y = exit.destination_y;
        self.facing = exit.facing();
        self.event_state.map_word=exit.map_word;
        self.event_state.party_x=self.x;self.event_state.party_y=self.y;
        let leader=self.event_state.leader_index();
        self.event_state.actors[leader].x=self.x;self.event_state.actors[leader].y=self.y;
        self.event_state.actors[leader].facing=self.facing as u8;
        self.init_field_npcs();
    }

    fn update_menu(&mut self, input: Input) {
        match self.menu_page {
            MenuPage::Main => {
                if input.up {
                    self.menu_cursor = (self.menu_cursor + MENU_ITEMS.len() - 1) % MENU_ITEMS.len();
                } else if input.down {
                    self.menu_cursor = (self.menu_cursor + 1) % MENU_ITEMS.len();
                } else if input.cancel || input.menu {
                    self.scene = Scene::Field;
                } else if input.confirm {
                    self.menu_page = match self.menu_cursor {
                        0 => MenuPage::Items,
                        1 => MenuPage::Magic,
                        2 => MenuPage::Equipment,
                        3 => MenuPage::Status,
                        4 => MenuPage::Save,
                        5 => MenuPage::Load,
                        6 => MenuPage::Config,
                        _ => {
                            self.scene = Scene::Field;
                            return;
                        }
                    };
                }
            }
            MenuPage::Items => {
                if input.cancel || input.menu {
                    self.menu_page = MenuPage::Main;
                } else if input.confirm && self.potions > 0 && self.hp < self.max_hp {
                    self.potions -= 1;
                    self.hp = (self.hp + 50).min(self.max_hp);
                    self.last_action = String::from("USED HI-POTION.");
                    self.menu_page = MenuPage::Main;
                }
            }
            MenuPage::Save => {
                let saved = self.save_state().is_ok();
                self.menu_page = MenuPage::Main;
                self.last_action = if saved {
                    String::from("GAME SAVED TO SLOT 1.")
                } else {
                    String::from("COULD NOT WRITE SLOT 1.")
                };
            }
            MenuPage::Load => {
                let loaded = self.load_state().is_ok();
                self.scene = Scene::Field;
                self.last_action = if loaded {
                    String::from("LOADED SLOT 1.")
                } else {
                    String::from("NO VALID SAVE IN SLOT 1.")
                };
            }
            MenuPage::Magic | MenuPage::Equipment | MenuPage::Status | MenuPage::Config => {
                if input.confirm || input.cancel || input.menu {
                    self.menu_page = MenuPage::Main;
                }
            }
        }
    }

    fn update_battle(&mut self, input: Input) {
        let Some((has_message, victory, escaped, cursor)) = self.battle.as_ref().map(|battle| {
            (battle.message.is_some(), battle.victory, battle.escaped, battle.cursor)
        }) else {
            self.scene = Scene::Field;
            return;
        };
        if has_message {
            if input.confirm {
                if victory {
                    self.battle = None;
                    self.scene = Scene::Field;
                    self.exp += 30;
                    self.gil += 50;
                    self.last_action = String::from("THE PARTY WON. GAINED 30 EXP AND 50 GIL.");
                    if self.exp >= 100 {
                        self.level = self.level.saturating_add(1);
                        self.exp -= 100;
                        self.max_hp += 8;
                        self.hp = self.max_hp;
                        self.max_mp += 2;
                        self.mp = self.max_mp;
                    }
                } else if escaped {
                    self.battle = None;
                    self.scene = Scene::Field;
                    self.y = 2;
                    self.last_action = String::from("THE PARTY ESCAPED.");
                } else {
                    if let Some(battle) = &mut self.battle {
                        battle.message = None;
                    }
                }
            }
            return;
        }
        if input.up {
            if let Some(battle) = &mut self.battle {
                battle.cursor = (cursor + BATTLE_COMMANDS.len() - 1) % BATTLE_COMMANDS.len();
            }
        } else if input.down {
            if let Some(battle) = &mut self.battle {
                battle.cursor = (cursor + 1) % BATTLE_COMMANDS.len();
            }
        } else if input.confirm {
            self.execute_battle_command();
        }
    }

    fn execute_battle_command(&mut self) {
        let Some(cursor) = self.battle.as_ref().map(|battle| battle.cursor) else { return };
        let player_message = match cursor {
            0 => {
                let damage = 16 + self.level as i32 * 3;
                if let Some(battle) = &mut self.battle {
                    battle.enemy_hp = (battle.enemy_hp - damage).max(0);
                }
                format!("TERRA ATTACKS FOR {damage} DAMAGE.")
            }
            1 => {
                if self.mp >= 4 {
                    self.mp -= 4;
                    let damage = 42;
                    if let Some(battle) = &mut self.battle {
                        battle.enemy_hp = (battle.enemy_hp - damage).max(0);
                    }
                    format!("CAST FIRE! {damage} DAMAGE.")
                } else {
                    String::from("NOT ENOUGH MP.")
                }
            }
            2 => {
                if self.potions > 0 {
                    self.potions -= 1;
                    self.hp = (self.hp + 50).min(self.max_hp);
                    String::from("USED HI-POTION.")
                } else {
                    String::from("NO HI-POTIONS LEFT.")
                }
            }
            _ => {
                let success = self.random(100) < 65;
                if success {
                    if let Some(battle) = &mut self.battle {
                        battle.escaped = true;
                        battle.message = Some(String::from("THE PARTY RAN AWAY!"));
                    }
                    return;
                }
                String::from("COULD NOT ESCAPE.")
            }
        };

        let Some(enemy_hp) = self.battle.as_ref().map(|battle| battle.enemy_hp) else { return };
        if enemy_hp <= 0 {
            if let Some(battle) = &mut self.battle {
                battle.victory = true;
                battle.message = Some(format!("{player_message} YMIR FALLS!"));
            }
            return;
        }

        let damage = 8 + (self.random(10) as i32);
        self.hp = (self.hp - damage).max(0);
        if self.hp == 0 {
            self.hp = self.max_hp / 2;
            self.mp = self.max_mp / 2;
            self.scene = Scene::Field;
            self.battle = None;
            self.y = 2;
            self.last_action = String::from("THE PARTY RETREATED TO RECOVER.");
            return;
        }
        if let Some(battle) = &mut self.battle {
            battle.message = Some(format!("{player_message} YMIR HITS FOR {damage}!"));
        }
    }

    fn draw_title(&self, canvas: &mut Canvas) {
        canvas.clear(0x00071125);
        for y in 0..150 {
            let shade = 28 + ((y * 35) / 150) as u32;
            let color = (shade << 16) | ((shade + 18) << 8) | (shade + 45);
            canvas.rect(0, y, WIDTH as i32, 1, color);
        }
        for i in 0..13 {
            let x = ((i * 47 + 19) % WIDTH) as i32;
            let y = ((i * 31 + 13) % 125) as i32;
            let glow = if (self.elapsed * 2.0 + i as f32).sin() > 0.5 { WHITE } else { PALE_BLUE };
            canvas.pixel(x, y, glow);
            canvas.pixel(x + 1, y, 0x0076A2D3);
        }
        canvas.line(0, 150, 78, 105, 0x002A4D77);
        canvas.line(78, 105, 143, 151, 0x00365D88);
        canvas.line(128, 152, 198, 91, 0x00233E66);
        canvas.line(198, 91, 255, 147, 0x003B6691);
        canvas.rect(0, 153, 256, 71, 0x00D5E7F5);
        for i in 0..34 {
            let x = ((i * 73 + 11) % WIDTH) as i32;
            let y = 157 + ((i * 19 + 7) % 64) as i32;
            canvas.pixel(x, y, 0x00F8FBFF);
        }
        canvas.centered_text(46, "FINAL FANTASY III", PALE_BLUE);
        canvas.panel(50, 179, 156, 30);
        canvas.centered_text(191, "PRESS START", WHITE);
    }

    fn draw_intro(&self, canvas: &mut Canvas) {
        if let Some((map_id, x, y)) = INTRO_MAPS.get(self.intro_index).copied() {
            if let Some(map) = self.assets.field_maps.get(&map_id) {
                map.draw(canvas, x, y);
            } else {
                canvas.clear(BLACK);
                canvas.centered_text(90, "OPENING MAP ASSET NOT BUILT", WHITE);
            }
        } else {
            canvas.clear(BLACK);
        }
        canvas.panel(8, 160, 240, 60);
        if let Some(lines) = INTRO.get(self.intro_index) {
            for (index, line) in lines.iter().enumerate() {
                canvas.centered_text(168 + index as i32 * 10, line, WHITE);
            }
        }
        canvas.text(222, 210, "Z", GOLD);
    }

    fn draw_field(&self, canvas: &mut Canvas) {
        if let Some(map) = self.assets.field_maps.get(&self.map_id) {
            if self.event_vm.is_some() {
                let actor = self.event_state.actors[self.event_state.leader_index()];
                map.draw_camera(canvas, map.camera_for_pixel(i32::from(actor.x) * 16 + i32::from(actor.offset_x).div_euclid(256),
                    i32::from(actor.y) * 16 + i32::from(actor.offset_y).div_euclid(256)));
            } else {
                let (x,y) = self.player_world_pixel();
                map.draw_camera(canvas,map.camera_for_pixel(x,y));
            }
        } else {
            canvas.clear(BLACK);
            canvas.centered_text(92, "FIELD MAP ASSETS NOT BUILT", WHITE);
            canvas.centered_text(108, "RUN TOOLS/EXTRACT_NATIVE_FIELD.PY", PALE_BLUE);
        }
        if self.event_vm.is_some() { self.draw_event_actors(canvas); }
        else { self.draw_player(canvas); }
        if let Some(dialogue) = &self.dialogue {
            self.draw_dialogue(canvas, dialogue);
        }
    }

    fn draw_event_actors(&self, canvas: &mut Canvas) {
        let Some(map) = self.assets.field_maps.get(&self.map_id) else { return };
        let leader = self.event_state.actors[self.event_state.leader_index()];
        let position = |actor: crate::event_vm::EventActor| (i32::from(actor.x) * 16 + i32::from(actor.offset_x).div_euclid(256),
            i32::from(actor.y) * 16 + i32::from(actor.offset_y).div_euclid(256));
        let (lx, ly) = position(leader);
        let (cx, cy) = map.camera_for_pixel(lx, ly);
        let mut visible = self.event_vm.as_ref().unwrap().actor_order().iter().copied()
            .filter(|id|self.event_state.actors[*id].enabled && self.event_state.actors[*id].visible)
            .map(|id|id as u8).collect::<Vec<_>>();
        visible.sort_by_key(|id| position(self.event_state.actors[usize::from(*id)]).1);
        for id in visible {
            let actor = self.event_state.actors[usize::from(id)];
            let (px, py) = position(actor);
            let walking = actor.offset_x != 0 || actor.offset_y != 0;
            let coordinate = if actor.offset_x != 0 { px } else { py };
            let animation = if walking { [1,2,1,0][crate::player_motion::patched_walk_phase(coordinate,actor.facing)] } else { 1 };
            let (frame, flip) = match actor.facing {
                0 => (WALK_UP[animation], false),
                1 => (WALK_LEFT[animation], true),
                2 => (WALK_DOWN[animation], false),
                _ => (WALK_LEFT[animation], false),
            };
            let sheet=self.assets.actor_sprites.get(&(actor.graphic,actor.palette))
                .or_else(||if id==0 {self.assets.terra.as_ref()}else if id==14||id==15 {self.assets.soldier.as_ref()}else{None});
            if actor.vehicle & 0x60 == 0x40 {
                if let Some(magitek) = &self.assets.magitek {
                    magitek.draw(canvas, sheet, actor.facing, (px, py), (px-cx, py-cy),
                        actor.offset_x != 0, walking, actor.vehicle & 0x80 != 0);
                    continue;
                }
            }
            self.draw_sprite(canvas, sheet, frame, px - cx, py - cy - 8, flip, 0x00AF6072);
        }
    }

    fn draw_player(&self, canvas: &mut Canvas) {
        let (world_x,world_y) = self.player_world_pixel();
        let (camera_x,camera_y) = self.assets.field_maps.get(&self.map_id)
            .map(|map| map.camera_for_pixel(world_x,world_y)).unwrap_or((0,0));
        let (sx,sy) = (world_x-camera_x,world_y-camera_y-8);
        let coordinate = if self.motion.active && matches!(self.facing,1|3) { world_x } else { world_y };
        let animation_frame = if self.motion.active {
            [1,2,1,0][crate::player_motion::patched_walk_phase(coordinate,self.facing as u8)]
        } else { 1 };
        let (frame, flip) = match self.facing {
            0 => (WALK_UP[animation_frame], false),
            1 => (WALK_LEFT[animation_frame], true),
            2 => (WALK_DOWN[animation_frame], false),
            _ => (WALK_LEFT[animation_frame], false),
        };
        let vehicle = self.event_state.actors[0].vehicle;
        if vehicle & 0x60 == 0x40 {
            if let Some(magitek) = &self.assets.magitek {
                magitek.draw(canvas, self.assets.terra.as_ref(), self.facing as u8,
                    (world_x,world_y), (sx, sy+8),
                    self.motion.active && matches!(self.facing, 1 | 3), self.motion.active, vehicle & 0x80 != 0);
                return;
            }
        }
        self.draw_sprite(canvas, self.assets.terra.as_ref(), frame, sx, sy, flip, 0x00AF6072);
    }

    fn draw_sprite(
        &self,
        canvas: &mut Canvas,
        sheet: Option<&SpriteSheet>,
        tile_indices: [usize; 6],
        x: i32,
        y: i32,
        flip: bool,
        fallback: u32,
    ) {
        if let Some(sprite) = sheet.filter(|s| tile_indices.iter().all(|&i| i < s.tiles.len())) {
            canvas.sprite_16x24(sprite, tile_indices, x, y, flip);
        } else {
            canvas.rect(x + 4, y + 1, 8, 6, 0x00E9C5A3);
            canvas.rect(x + 3, y + 7, 10, 11, fallback);
            canvas.rect(x + 3, y + 18, 4, 6, 0x00283348);
            canvas.rect(x + 9, y + 18, 4, 6, 0x00283348);
        }
    }

    fn draw_menu(&self, canvas: &mut Canvas) {
        canvas.panel(15, 30, 226, 130);
        match self.menu_page {
            MenuPage::Main => {
                canvas.text(27, 40, "TERRA", GOLD);
                for (index, item) in MENU_ITEMS.iter().enumerate() {
                    let y = 55 + index as i32 * 12;
                    let marker = if index == self.menu_cursor { ">" } else { " " };
                    canvas.text(29, y, marker, GOLD);
                    canvas.text(40, y, item, WHITE);
                }
                canvas.text(130, 58, "LEVEL", PALE_BLUE);
                canvas.text(184, 58, &self.level.to_string(), WHITE);
                canvas.text(130, 74, "EXP", PALE_BLUE);
                canvas.text(184, 74, &self.exp.to_string(), WHITE);
                canvas.text(130, 90, "NEXT", PALE_BLUE);
                let next=self.character_tables.next_level_exp(self.level)
                    .map(|exp|exp.saturating_sub(self.exp).to_string()).unwrap_or_else(||"---".into());
                canvas.text(184, 90, &next, WHITE);
                canvas.text(130, 114, "GIL", PALE_BLUE);
                canvas.text(160, 114, &self.gil.to_string(), WHITE);
            }
            MenuPage::Items => {
                canvas.text(29, 43, "ITEMS", GOLD);
                canvas.text(35, 64, "HI-POTION", WHITE);
                canvas.text(169, 64, &format!("X {}", self.potions), PALE_BLUE);
                canvas.text(35, 88, "RECOVERS 50 HP", PALE_BLUE);
                canvas.text(35, 123, "Z USE   X BACK", 0x008EABC8);
            }
            MenuPage::Magic => {
                canvas.text(29, 43, "MAGIC", GOLD);
                canvas.text(35, 64, "FIRE", WHITE);
                canvas.text(35, 80, "CURE", WHITE);
                canvas.text(35, 112, "MAGIC LIST IS NOT YET MAPPED", 0x008EABC8);
            }
            MenuPage::Equipment => {
                canvas.text(29, 43, "EQUIPMENT", GOLD);
                if let Some(terra)=&self.characters[0] {
                    for (i,(label,id)) in ["R HAND","L HAND","HEAD","BODY","RELIC 1","RELIC 2"].iter().zip(terra.equipment()).enumerate() {
                        let y=64+i as i32*13;
                        canvas.text(35,y,label,PALE_BLUE);canvas.text(100,y,self.item_table.name(id),WHITE);
                    }
                }
            }
            MenuPage::Status => {
                canvas.text(29, 43, "STATUS", GOLD);
                canvas.text(35, 58, "TERRA", WHITE);
                canvas.text(35, 70, &format!("LEVEL {}  EXP {}", self.level, self.exp), WHITE);
                canvas.text(35, 82, &format!("HP {} / {}", self.hp, self.max_hp), WHITE);
                canvas.text(35, 94, &format!("MP {} / {}", self.mp, self.max_mp), WHITE);
                if let Some(terra)=&self.characters[0] {
                    if let Ok(stats)=self.item_table.calculate(terra,terra.equipment(),0) {
                        for (i,(a,b)) in [(format!("VIG {:3}",stats.vigor()),format!("STA {:3}",stats.stamina())),
                            (format!("SPD {:3}",stats.speed()),format!("MAG {:3}",stats.magic())),
                            (format!("DEF {:3}",stats.defense()),format!("MDEF {:3}",stats.magic_defense())),
                            (format!("EVA {:3}",stats.evade()),format!("MBLK {:3}",stats.magic_block()))].iter().enumerate() {
                            canvas.text(35,110+i as i32*12,a,WHITE);canvas.text(135,110+i as i32*12,b,WHITE);
                        }
                    }
                }
            }
            MenuPage::Save | MenuPage::Load | MenuPage::Config => {
                let title = match self.menu_page {
                    MenuPage::Save => "SAVE",
                    MenuPage::Load => "LOAD",
                    _ => "CONFIG",
                };
                canvas.text(29, 43, title, GOLD);
                canvas.text(35, 66, "PRESS Z TO CONTINUE", WHITE);
                canvas.text(35, 88, "X RETURNS TO MENU", PALE_BLUE);
            }
        }
    }

    fn draw_battle(&self, canvas: &mut Canvas) {
        canvas.clear(0x000A1830);
        canvas.rect(0, 0, 256, 114, 0x00142646);
        for i in 0..8 {
            canvas.line(i * 36 - 30, 0, 128, 113, 0x00223A5B);
        }
        canvas.rect(0, 111, 256, 53, 0x00667581);
        canvas.rect(0, 116, 256, 48, 0x008F9AA0);
        canvas.line(0, 139, 256, 128, 0x00D0D5D8);
        canvas.panel(26, 25, 204, 45);
        canvas.centered_text(36, "YMIR", GOLD);
        if let Some(battle) = &self.battle {
            let width = (battle.enemy_hp.max(0) * 150 / battle.enemy_max_hp).max(0);
            canvas.rect(53, 53, 150, 6, 0x00463640);
            canvas.rect(53, 53, width, 6, RED);
        }
        canvas.rect(104, 86, 50, 29, 0x00CC8D77);
        canvas.rect(111, 78, 35, 17, 0x00E4C3A2);
        canvas.rect(98, 110, 17, 7, 0x00A6D3EB);
        canvas.rect(140, 110, 17, 7, 0x00A6D3EB);
        canvas.panel(4, 164, 248, 60);
        canvas.text(12, 171, "TERRA", GOLD);
        canvas.text(12, 184, &format!("HP {:03}/{:03}   MP {:02}/{:02}", self.hp, self.max_hp, self.mp, self.max_mp), WHITE);
        canvas.text(12, 204, &format!("GIL {:06}", self.gil), PALE_BLUE);
        canvas.panel(169, 166, 78, 54);
        for (index, command) in BATTLE_COMMANDS.iter().enumerate() {
            let y = 171 + index as i32 * 11;
            canvas.text(175, y, if self.battle.as_ref().is_some_and(|b| b.cursor == index) { ">" } else { " " }, GOLD);
            canvas.text(186, y, command, WHITE);
        }
        if let Some(battle) = &self.battle {
            if let Some(message) = battle.message.as_ref() {
                canvas.rect(0, 120, 256, 40, 0x0008111F);
                canvas.text(10, 132, message, WHITE);
                canvas.text(10, 149, "PRESS Z", PALE_BLUE);
            }
        }
    }

    fn draw_dialogue(&self, canvas: &mut Canvas, dialogue: &Dialogue) {
        canvas.panel(8, 103, 240, 61);
        if let Some(line) = dialogue.lines.get(dialogue.current) {
            for (index, wrapped) in wrap_text(line, 35).iter().take(5).enumerate() {
                canvas.text(18, 108 + index as i32 * 9, wrapped, WHITE);
            }
        }
        canvas.text(218, 154, "Z", GOLD);
    }

    fn walkable(&self, x: i16, y: i16, dx: i16, dy: i16) -> bool {
        self.assets
            .field_maps
            .get(&self.map_id)
            .map(|map| map.can_move(self.x, self.y, x, y))
            .unwrap_or(x >= 0 && y >= 0 && x < 64 && y < 64 && (dx != 0 || dy != 0))
    }

    fn begin_battle(&mut self) {
        self.scene = Scene::Battle;
        self.battle = Some(Battle {
            cursor: 0,
            enemy_hp: 180,
            enemy_max_hp: 180,
            message: None,
            victory: false,
            escaped: false,
        });
    }

    fn dialogue(&mut self, lines: &[&str]) {
        self.script_dialogue(lines, DialogueCompletion::ResumeField);
    }

    fn script_dialogue(&mut self, lines: &[&str], completion: DialogueCompletion) {
        self.dialogue = Some(Dialogue {
            lines: lines.iter().map(|line| (*line).to_owned()).collect(),
            current: 0,
            completion,
        });
    }

    fn complete_dialogue(&mut self, completion: DialogueCompletion) {
        match completion {
            DialogueCompletion::ResumeField => {}
            DialogueCompletion::EnterNarshe => {
                self.start_opening_event(self.gil);
            }
            DialogueCompletion::ResumeEvent => {self.event_dialogue=None;self.advance_event();},
        }
    }

    fn init_field_npcs(&mut self) {
        self.assets.field_events.init_npcs(self.map_id,&mut self.event_state);
        self.npc_map=Some(self.map_id);
    }

    fn start_field_event(&mut self,entry:usize) {
        self.pending_exit=false;
        if self.npc_map!=Some(self.map_id) {self.init_field_npcs();}
        self.event_state.map_word=(self.event_state.map_word&!0x1ff)|self.map_id;
        self.event_state.party_x=self.x;self.event_state.party_y=self.y;self.event_state.gil=self.gil;
        let leader=self.event_state.leader_index();
        if let Some(actor)=self.event_state.actors.get_mut(leader) {
            actor.x=self.x;actor.y=self.y;actor.offset_x=0;actor.offset_y=0;actor.facing=self.facing as u8;
            if !actor.enabled {actor.enabled=true;actor.visible=true;actor.speed=2;actor.graphic=0;actor.palette=2;}
        }
        let mut vm=EventVm::scheduled(entry);vm.frame=self.field_frame.saturating_sub(1);
        self.event_vm=Some(vm);self.event_clock=self.field_clock;self.field_clock=0.0;
        self.event_fault=false;self.event_dialogue=None;
        self.advance_event();
    }

    fn start_opening_event(&mut self, initial_gil: u32) {
        let switches = self.event_state.switches;
        self.event_state = EventState::opening_context();
        self.event_state.switches = switches;
        self.event_state.gil = initial_gil;
        self.opening_phase = 2;
        self.event_vm = Some(EventVm::scheduled(NARSHE_ENTRY));
        self.event_clock = 0.0;
        self.event_fault = false;
        self.advance_event();
    }

    fn advance_event(&mut self) {
        if self.event_fault { return; }
        let Some(mut vm) = self.event_vm.take() else { return };
        self.event_dialogue=None;
        let previous_map=self.map_id;
        let result = vm.step_frame(&self.assets.event_script, &mut self.event_state);
        self.map_id = self.event_state.map_word & 0x01ff;
        self.gil = self.event_state.gil;
        if previous_map!=self.map_id || self.npc_map!=Some(self.map_id) {self.init_field_npcs();vm.refresh_actor_order(&self.event_state);}
        if let Some(actor) = self.event_state.actors.get(self.event_state.leader_index()) {
            self.x = actor.x;
            self.y = actor.y;
            self.facing = usize::from(actor.facing);
        }
        match result {
            Ok(EventYield::Pending) => self.event_vm = Some(vm),
            Ok(EventYield::Dialogue { id, flags }) => {
                self.event_dialogue=Some((id,flags));
                self.event_vm = Some(vm);
                match self.assets.dialogue_catalog.literal(id) {
                    Ok(text) => self.script_dialogue(&[&text], DialogueCompletion::ResumeEvent),
                    Err(error) => {
                        eprintln!("{error}");
                        self.last_action = error;
                        self.dialogue(&["THIS EVENT'S TEXT CANNOT BE DISPLAYED YET."]);
                    }
                }
            }
            Ok(EventYield::Battle {group,flags}) => {
                self.event_vm=Some(vm);self.event_fault=true;self.event_dialogue=None;
                self.last_action=format!("TARGET EVENT BATTLE {group} / FLAGS {flags:02X} IS NOT IMPLEMENTED YET.");
                self.dialogue(&[&format!("TARGET BATTLE GROUP {group} IS NOT IMPLEMENTED YET.")]);
            }
            Ok(EventYield::Returned) => {
                self.event_dialogue=None;
                self.opening_phase = 3;
                self.field_frame = vm.frame;
                self.field_clock = 0.0;
            },
            Err(error) => {
                eprintln!("{error}");
                self.last_action = error.to_string();
                self.event_fault = true;
                self.event_vm = Some(vm);
                self.dialogue(&["THIS EVENT CANNOT CONTINUE YET."]);
            }
        }
    }

    /// Advance one actual player step, releasing direction after it starts.
    fn walk_one_tile(&mut self, input: Input) -> std::io::Result<()> {
        let initial = self.steps;
        for _ in 0..120 {
            self.update(if self.steps == initial { input } else { Input::default() }, Duration::from_secs_f64(1.0/60.0));
            if self.steps != initial && !self.motion.active {
                // One more tick observes the aligned entrance before movement.
                self.update(Input::default(),Duration::from_secs_f64(1.0/60.0));
                return Ok(());
            }
        }
        Err(std::io::Error::new(std::io::ErrorKind::TimedOut,"player step did not complete"))
    }

    /// Diagnostics advance logical time without feeding confirmation input.
    fn settle_event(&mut self) -> std::io::Result<()> {
        for _ in 0..36000 {
            if self.event_vm.is_none() || self.dialogue.is_some() { return Ok(()); }
            self.update(Input::default(), Duration::from_secs_f64(1.0 / 60.0));
        }
        Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "event did not reach a dialogue or return"))
    }

    fn random(&mut self, upper: u32) -> u32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        if upper == 0 { 0 } else { self.rng % upper }
    }

    fn save_state(&self) -> std::io::Result<()> {
        fs::create_dir_all("save")?;
        fs::write("save/slot1.dat", self.save_text())
    }

    fn save_text(&self) -> String {
        // Existing field menus still project Terra through the scalar fields.
        // Persist that projection together with every initialized guest.
        let mut characters=self.characters.clone();
        if let Some(terra)=characters[0].as_mut() {
            terra.hp=self.hp as u16;terra.max_hp=self.max_hp as u16;
            terra.mp=self.mp as u16;terra.max_mp=self.max_mp as u16;
            terra.level=self.level;terra.exp=self.exp;
        }
        let character_snapshot=crate::characters::encode_roster(&characters);
        let opening_line = self.dialogue.as_ref()
            .filter(|dialogue| dialogue.completion != DialogueCompletion::ResumeField)
            .map(|dialogue| dialogue.current).unwrap_or(0);
        let switches = self.event_state.switches.iter().map(|byte| format!("{byte:02x}"))
            .collect::<Vec<_>>().join("");
        let snapshot=crate::event_vm::encode_snapshot(&self.event_state,self.event_vm.as_ref()).iter()
            .map(|byte|format!("{byte:02x}")).collect::<String>();
        let npc_map=self.npc_map.map(|map|map.to_string()).unwrap_or_default();
        let event_dialogue=self.event_dialogue.map(|(id,flags)|format!("{id},{flags}")).unwrap_or_default();
        let vehicles = self.event_state.actors.iter().map(|actor| format!("{:02x}", actor.vehicle)).collect::<String>();
        format!(
            "FF6NATIVE9\ncharacters={}\nevent_snapshot={}\nevent_fault={}\nevent_clock={}\nevent_text={}\nnpc_map={}\nfacing={}\nfield_frame={}\nfield_clock={}\nfield_motion={},{},{},{}\nfield_pending_exit={}\nevent_ticks={}\nevent_dialogue={}\nmap_id={}\nopening_phase={}\nopening_line={}\nx={}\ny={}\nmax_hp={}\nhp={}\nmax_mp={}\nmp={}\npotions={}\ngil={}\nexp={}\nlevel={}\nsteps={}\nevent_switches={}\nevent_vehicles={}\n",
            character_snapshot,snapshot,u8::from(self.event_fault),self.event_clock,event_dialogue,npc_map,
            self.facing,self.field_frame,self.field_clock,self.motion.direction,self.motion.speed,self.motion.progress,
            u8::from(self.motion.active),u8::from(self.pending_exit),
            self.event_vm.as_ref().map(|vm| vm.frame).unwrap_or(0), u8::from(self.dialogue.is_some()),
            self.map_id, self.opening_phase, opening_line, self.x, self.y, self.max_hp, self.hp, self.max_mp, self.mp, self.potions, self.gil, self.exp, self.level, self.steps, switches, vehicles
        )
    }

    fn load_state(&mut self) -> std::io::Result<()> {
        let data = fs::read_to_string("save/slot1.dat")?;
        self.restore_text(&data)
    }

    fn restore_text(&mut self, data: &str) -> std::io::Result<()> {
        let mut lines = data.lines();
        let signature=lines.next();
        if !matches!(signature, Some("FF6NATIVE2") | Some("FF6NATIVE3") | Some("FF6NATIVE4") | Some("FF6NATIVE5") | Some("FF6NATIVE6") | Some("FF6NATIVE7") | Some("FF6NATIVE8") | Some("FF6NATIVE9")) {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "bad save signature"));
        }
        let mut opening_line = 0usize;
        let mut event_ticks: Option<u64> = None;
        let mut event_dialogue = false;
        let mut saved_switches = None;
        let mut saved_vehicles = None;
        let mut saved_motion = None;
        let mut saved_field_frame = 0u64;
        let mut saved_field_clock = 0.0f64;
        let mut saved_pending_exit = false;
        let mut saved_snapshot=None;
        let mut saved_characters=None;
        let mut saved_fault=false;
        let mut saved_event_clock=0.0f64;
        let mut saved_event_dialogue=None;
        let mut saved_npc_map:Option<Option<u16>>=None;
        for line in lines {
            let Some((key, value)) = line.split_once('=') else { continue };
            match key {
                "characters" => saved_characters=Some(crate::characters::decode_roster(value)
                    .map_err(|error|std::io::Error::new(std::io::ErrorKind::InvalidData,error))?),
                "npc_map" => {
                    saved_npc_map=Some(if value.is_empty(){None}else{Some(value.parse::<u16>().ok().filter(|map|*map<512)
                        .ok_or_else(||std::io::Error::new(std::io::ErrorKind::InvalidData,"bad NPC map id"))?)});
                }
                "event_snapshot" => {
                    if value.len()>32768 || value.len()%2!=0 || !value.is_ascii() {return Err(std::io::Error::new(std::io::ErrorKind::InvalidData,"bad event snapshot hex"));}
                    let bytes=(0..value.len()).step_by(2).map(|i|u8::from_str_radix(&value[i..i+2],16)).collect::<Result<Vec<_>,_>>()
                        .map_err(|_|std::io::Error::new(std::io::ErrorKind::InvalidData,"bad event snapshot byte"))?;
                    saved_snapshot=Some(crate::event_vm::decode_snapshot(&self.assets.event_script,&bytes)
                        .map_err(|error|std::io::Error::new(std::io::ErrorKind::InvalidData,error))?);
                }
                "event_fault" => saved_fault=value=="1",
                "event_clock" => {
                    saved_event_clock=value.parse().map_err(|_|std::io::Error::new(std::io::ErrorKind::InvalidData,"bad event clock"))?;
                    if !saved_event_clock.is_finite() || !(0.0..64.0).contains(&saved_event_clock) {return Err(std::io::Error::new(std::io::ErrorKind::InvalidData,"event clock outside supported range"));}
                }
                "event_text" if !value.is_empty() => {
                    let Some((id,flags))=value.split_once(',') else {return Err(std::io::Error::new(std::io::ErrorKind::InvalidData,"bad event dialogue"));};
                    saved_event_dialogue=Some((id.parse::<u16>().map_err(|_|std::io::Error::new(std::io::ErrorKind::InvalidData,"bad dialogue id"))?,
                        flags.parse::<u16>().map_err(|_|std::io::Error::new(std::io::ErrorKind::InvalidData,"bad dialogue flags"))?));
                }
                "facing" => {
                    self.facing = value.parse().ok().filter(|value| *value <= 3)
                        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData,"bad facing"))?;
                }
                "field_frame" => saved_field_frame = value.parse().map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData,"bad field frame"))?,
                "field_clock" => {
                    saved_field_clock = value.parse().map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData,"bad field clock"))?;
                    if !saved_field_clock.is_finite() || !(0.0..1.0).contains(&saved_field_clock) {
                        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData,"field clock out of range"));
                    }
                }
                "field_pending_exit" => saved_pending_exit = value == "1",
                "field_motion" => {
                    let values = value.split(',').map(str::parse::<u16>).collect::<Result<Vec<_>,_>>()
                        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData,"bad field motion"))?;
                    if values.len() != 4 || values[0] > 3 || values[1] > 4 || values[3] > 1 {
                        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData,"bad field motion components"));
                    }
                    saved_motion = Some(crate::player_motion::PlayerMotion::restore(values[0] as u8,values[1] as u8,values[2],values[3] != 0)
                        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData,"unaligned field motion"))?);
                }
                "event_ticks" => event_ticks = value.parse().ok(),
                "event_dialogue" => event_dialogue = value == "1",
                "map_id" => self.map_id = value.parse().unwrap_or(self.map_id),
                "opening_phase" => self.opening_phase = value.parse().unwrap_or(self.opening_phase),
                "opening_line" => opening_line = value.parse().unwrap_or(0),
                "event_vehicles" => {
                    if value.len() != 0x35*2 || !value.is_ascii() {
                        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "bad vehicle state"));
                    }
                    let mut vehicles = [0; 0x35];
                    for (i, vehicle) in vehicles.iter_mut().enumerate() {
                        *vehicle = u8::from_str_radix(&value[i*2..i*2+2],16)
                            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "bad vehicle byte"))?;
                    }
                    saved_vehicles = Some(vehicles);
                }
                "event_switches" => {
                    let invalid = || std::io::Error::new(std::io::ErrorKind::InvalidData, "bad event-switch data");
                    if value.len() != SWITCH_BYTES * 2 || !value.is_ascii() { return Err(invalid()); }
                    let mut switches = [0; SWITCH_BYTES];
                    for (i, byte) in switches.iter_mut().enumerate() {
                        *byte = u8::from_str_radix(&value[i*2..i*2+2], 16).map_err(|_| invalid())?;
                    }
                    saved_switches = Some(switches);
                }
                "x" => self.x = value.parse().unwrap_or(self.x),
                "y" => self.y = value.parse().unwrap_or(self.y),
                "hp" => self.hp = value.parse().unwrap_or(self.hp).clamp(1, self.max_hp),
                "max_hp" => self.max_hp = value.parse().unwrap_or(self.max_hp).clamp(1, 9999),
                "mp" => self.mp = value.parse().unwrap_or(self.mp).clamp(0, self.max_mp),
                "max_mp" => self.max_mp = value.parse().unwrap_or(self.max_mp).clamp(0, 999),
                "potions" => self.potions = value.parse().unwrap_or(self.potions),
                "gil" => self.gil = value.parse().unwrap_or(self.gil),
                "exp" => self.exp = value.parse().unwrap_or(self.exp),
                "level" => self.level = value.parse().unwrap_or(self.level),
                "steps" => self.steps = value.parse().unwrap_or(self.steps),
                _ => {}
            }
        }
        if signature==Some("FF6NATIVE9") && saved_characters.is_none() {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData,"missing character snapshot"));
        }
        if let Some(characters)=saved_characters {
            let terra=characters[0].as_ref().ok_or_else(||std::io::Error::new(std::io::ErrorKind::InvalidData,"saved Terra is missing"))?;
            self.hp=i32::from(terra.hp);self.max_hp=i32::from(terra.max_hp);
            self.mp=i32::from(terra.mp);self.max_mp=i32::from(terra.max_mp);
            self.level=terra.level;self.exp=terra.exp;self.characters=characters;
        } else {
            self.characters=self.character_tables.opening_party().map_err(std::io::Error::other)?;
            let terra=self.characters[0].as_mut().unwrap();
            terra.hp=self.hp as u16;terra.max_hp=self.max_hp as u16;
            terra.mp=self.mp as u16;terra.max_mp=self.max_mp as u16;
            terra.exp=self.exp;terra.level=self.level;
        }
        if let Some(map) = self.assets.field_maps.get(&self.map_id) {
            let (x, y) = map.clamp_position(self.x, self.y);
            self.x = x;
            self.y = y;
        }
        self.scene = Scene::Field;
        self.battle = None;
        self.motion = crate::player_motion::PlayerMotion::default();
        self.field_clock = 0.0;
        self.pending_exit = false;
        self.pending_menu = false;
        self.event_vm = None;
        self.npc_map=None;
        self.event_state = EventState::default();
        self.assets.field_events.initialize_switches(&mut self.event_state);
        if let Some(switches) = saved_switches {
            self.event_state.switches = switches;
        } else if self.opening_phase == 3 {
            // Legacy saves completed the same opening but did not store bits.
            self.event_state.set_switch(0x1c1, true);
        }
        self.event_fault=false;self.event_dialogue=None;
        if let Some((state,vm))=saved_snapshot {
            self.event_state=state;self.event_vm=vm;self.event_clock=saved_event_clock;
            self.event_fault=saved_fault;self.event_dialogue=saved_event_dialogue;self.npc_map=saved_npc_map.unwrap_or(Some(self.map_id));
            self.dialogue=None;
            if self.event_vm.is_some() {
                if let Some((id,_))=self.event_dialogue {
                    let text=self.assets.dialogue_catalog.literal(id).map_err(|error|std::io::Error::new(std::io::ErrorKind::InvalidData,error))?;
                    self.script_dialogue(&[&text],DialogueCompletion::ResumeEvent);
                } else if let Some((group,_))=self.event_vm.as_ref().and_then(|vm|vm.pending_battle) {
                    self.event_fault=true;self.dialogue(&[&format!("TARGET BATTLE GROUP {group} IS NOT IMPLEMENTED YET.")]);
                } else if self.event_fault {self.dialogue(&["THIS EVENT IS HALTED AT AN UNIMPLEMENTED COMMAND."]);}
            } else if self.opening_phase==1 {self.script_dialogue(&OPENING_DIALOGUE,DialogueCompletion::EnterNarshe);}
            else {self.dialogue(&["GAME LOADED."]);}
        } else {match self.opening_phase {
            1 => self.script_dialogue(&OPENING_DIALOGUE, DialogueCompletion::EnterNarshe),
            2 => {
                // The selected entry has one dialogue yield. Reconstruct its
                // paused actor/VM state from target bytes, then retain the saved
                // wallet so replay does not award gil a second time.
                let saved_gil = self.gil;
                self.start_opening_event(0);
                let target = event_ticks.filter(|ticks| *ticks != 0);
                if target.is_some_and(|ticks| ticks > 36000) {
                    return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "event replay exceeds supported limit"));
                }
                for _ in 0..36000 {
                    let ticks = self.event_vm.as_ref().map(|vm| vm.frame).unwrap_or(0);
                    if target.is_some_and(|end| ticks >= end) || (target.is_none() && self.dialogue.is_some()) { break; }
                    if self.event_vm.is_none() || self.event_fault { break; }
                    // During reconstruction only, a recorded dialogue was already
                    // acknowledged if subsequent event frames were saved.
                    self.dialogue = None;
                    self.advance_event();
                }
                if target.is_some() && !event_dialogue { self.dialogue = None; }
                self.gil = saved_gil;
                self.event_state.gil = saved_gil;
            }
            _ => self.dialogue(&["GAME LOADED."]),
        }}
        self.field_frame = saved_field_frame;
        self.field_clock = saved_field_clock;
        self.pending_exit = saved_pending_exit;
        if let Some(motion) = saved_motion {
            if motion.active {
                if self.opening_phase != 3 || self.pending_exit {
                    return Err(std::io::Error::new(std::io::ErrorKind::InvalidData,"motion conflicts with event/exit state"));
                }
                let (dx,dy) = [(0,-1),(1,0),(0,1),(-1,0)][usize::from(motion.direction)];
                if !self.assets.field_maps.get(&self.map_id).is_some_and(|map| map.in_bounds(self.x+dx,self.y+dy)) {
                    return Err(std::io::Error::new(std::io::ErrorKind::InvalidData,"motion destination outside map"));
                }
            }
            self.motion = motion;
        }
        if let Some(vehicles) = saved_vehicles {
            for (actor, vehicle) in self.event_state.actors.iter_mut().zip(vehicles) { actor.vehicle = vehicle; }
        } else if self.opening_phase == 3 {
            // Legacy prototype saves reached control without storing the
            // vehicle flags that remain active after `_cc9a4f` returns.
            self.event_state.actors[0].vehicle = 0xc0;
        }
        if self.dialogue.as_ref().is_some_and(|dialogue|dialogue.completion!=DialogueCompletion::ResumeField) {
            if let Some(dialogue) = &mut self.dialogue {
                dialogue.current = opening_line.min(dialogue.lines.len().saturating_sub(1));
            }
        }
        Ok(())
    }
}

/// Deterministic native-renderer captures. These are implementation frames,
/// not reference emulator frames or a validation of original event timing.
pub fn capture_equipment(directory:&std::path::Path)->std::io::Result<()> {
    use std::fmt::Write;
    fs::create_dir_all(directory)?;
    let mut game=Game::new();let mut canvas=Canvas::new();
    game.menu_page=MenuPage::Equipment;game.draw_menu(&mut canvas);
    canvas.save_bmp(&directory.join("opening-equipment-menu.bmp"))?;
    canvas.clear(BLACK);game.menu_page=MenuPage::Status;game.draw_menu(&mut canvas);
    canvas.save_bmp(&directory.join("opening-status-menu.bmp"))?;
    let mut rows=String::from("slot,vigor,speed,stamina,magic,defense,magic_defense,evade,magic_block,right_power,left_power,field_effects\n");
    for (slot,c) in game.characters.iter().enumerate() {if let Some(c)=c {
        let s=game.item_table.calculate(c,c.equipment(),0).map_err(std::io::Error::other)?;
        writeln!(rows,"{slot},{},{},{},{},{},{},{},{},{},{},{}",s.vigor(),s.speed(),s.stamina(),s.magic(),s.defense(),s.magic_defense(),s.evade(),s.magic_block(),s.raw[12],s.raw[13],s.field_effects()).unwrap();
    }}
    fs::write(directory.join("native-party-equipment.csv"),rows)
}

pub fn capture_characters(directory:&std::path::Path)->std::io::Result<()> {
    use std::fmt::Write;
    fs::create_dir_all(directory)?;
    let mut game=Game::new();
    let mut rows=String::from("state,slot,property,level,hp,mp,exp\n");
    for (slot,c) in game.characters.iter().enumerate() {if let Some(c)=c {
        writeln!(rows,"new,{slot},{},{},{},{},{}",c.property_id,c.level,c.hp,c.mp,c.exp).unwrap();
    }}
    if (game.hp,game.mp,game.exp,game.level)!=(63,27,96,3) {return Err(std::io::Error::other("Game scalar stats differ from target opening"));}
    let mut canvas=Canvas::new();game.draw_menu(&mut canvas);
    canvas.save_bmp(&directory.join("opening-stats-menu.bmp"))?;
    game.hp=17;game.mp=9;
    game.characters[14].as_mut().unwrap().hp=29;
    let saved=game.save_text();
    fs::write(directory.join("character-save.dat"),&saved)?;
    let mut loaded=Game::new();loaded.restore_text(&saved)?;
    loaded.dialogue=None; // Dismiss the local "GAME LOADED" notice.
    if loaded.save_text()!=saved {return Err(std::io::Error::other("Character/event snapshot round trip differs"));}
    for (slot,c) in loaded.characters.iter().enumerate() {if let Some(c)=c {
        writeln!(rows,"restored,{slot},{},{},{},{},{}",c.property_id,c.level,c.hp,c.mp,c.exp).unwrap();
    }}
    fs::write(directory.join("native-game-party.csv"),rows)
}

pub fn capture_opening(directory: &std::path::Path) -> std::io::Result<()> {
    let mut game = Game::new();
    for map_id in [18, 19, 20, 23, 244] {
        if !game.assets.field_maps.contains_key(&map_id) {
            return Err(std::io::Error::new(std::io::ErrorKind::NotFound,
                format!("Missing field map {map_id}; run tools/extract_native_field.py")));
        }
    }
    fs::create_dir_all(directory)?;
    let mut canvas = Canvas::new();
    let mut trace = String::from("frame,scene,render_map,x,y,opening_phase,dialogue_line,gil\n");
    let mut capture = |name: &str, game: &Game| -> std::io::Result<()> {
        game.draw(&mut canvas);
        canvas.save_bmp(&directory.join(format!("{name}.bmp")))?;
        let (map, x, y) = if game.scene == Scene::Intro {
            INTRO_MAPS[game.intro_index]
        } else {
            (game.map_id, game.x, game.y)
        };
        let line = game.dialogue.as_ref().map(|dialogue| dialogue.current.to_string())
            .unwrap_or_default();
        trace.push_str(&format!("{name},{:?},{map},{x},{y},{},{line},{}\n",
            game.scene, game.opening_phase, game.gil));
        Ok(())
    };
    let confirm = Input { confirm: true, ..Input::default() };
    let frame = Duration::from_millis(16);
    capture("00-title", &game)?;
    game.update(confirm, frame);
    for card in 0..4 {
        capture(&format!("0{}-lore", card + 1), &game)?;
        game.update(confirm, frame);
    }
    capture("05-biggs", &game)?;
    for _ in 0..OPENING_DIALOGUE.len() {
        game.update(confirm, frame);
    }
    game.settle_event()?;
    capture("06-wedge", &game)?;
    game.update(confirm, frame);
    game.settle_event()?;
    capture("07-player-control", &game)?;
    fs::write(directory.join("state-trace.csv"), trace)
}

/// Render the live opening scheduler at regular intervals and dialogue/return.
pub fn capture_opening_motion(directory: &std::path::Path) -> std::io::Result<()> {
    fs::create_dir_all(directory)?;
    let mut game = Game::new();
    game.scene = Scene::Field;
    game.start_opening_event(0);
    let mut canvas = Canvas::new();
    let mut rows = String::from("frame,event_offset,yield,object,x,y,offset_x,offset_y,visible,facing,gil\n");
    let mut captures = String::new();
    for frame in 1..36000u64 {
        let stage = if game.event_vm.is_none() { "returned" }
            else if game.dialogue.is_some() { "dialogue" } else { "pending" };
        let pc = game.event_vm.as_ref().map(|vm| vm.pc).unwrap_or(crate::event_vm::NARSHE_ENTRY_END);
        for id in [0, 14, 15] {
            let actor = game.event_state.actors[id];
            rows.push_str(&format!("{frame},0x{pc:06X},{stage},{id},{},{},{},{},{},{},{}\n",
                actor.x, actor.y, actor.offset_x, actor.offset_y, u8::from(actor.visible), actor.facing, game.gil));
        }
        if frame == 1 || frame % 32 == 0 || stage != "pending" {
            game.draw(&mut canvas);
            let filename = format!("{frame:04}-{stage}.bmp");
            canvas.save_bmp(&directory.join(&filename))?;
            captures.push_str(&format!("{filename}\n"));
        }
        if stage == "returned" {
            fs::write(directory.join("captures.txt"), captures)?;
            return fs::write(directory.join("game-timeline.csv"), rows);
        }
        game.update(Input { confirm: stage == "dialogue", ..Input::default() }, Duration::from_secs_f64(1.0 / 60.0));
    }
    Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "opening motion did not return"))
}

/// Actual player-triggered scenes; no enemy outcome or story flag is fabricated.
pub fn capture_trigger_scenes(directory:&std::path::Path)->std::io::Result<()> {
    fs::create_dir_all(directory)?;
    let make=||->std::io::Result<Game> {
        let mut game=Game::new();game.scene=Scene::Field;game.map_id=18;game.start_opening_event(0);
        game.settle_event()?;game.dialogue=None;game.complete_dialogue(DialogueCompletion::ResumeEvent);game.settle_event()?;
        Ok(game)
    };
    let mut summaries=String::from("stage,map,x,y,gil,party,dialogue,event_cpu,battle,flag_0128\n");
    let mut canvas=Canvas::new();
    let mut capture=|name:&str,game:&Game|->std::io::Result<()> {
        game.draw(&mut canvas);canvas.save_bmp(&directory.join(format!("{name}.bmp")))?;
        summaries.push_str(&format!("{name},{},{},{},{},{}:{}:{}:{},{},{},{},{}\n",game.map_id,game.x,game.y,game.gil,
            game.event_state.party[0],game.event_state.party[1],game.event_state.party[2],game.event_state.party[3],
            game.event_dialogue.map(|(id,_)|id.to_string()).unwrap_or_default(),
            game.event_vm.as_ref().map(|vm|format!("0x{:06X}",0xca0000+vm.pc)).unwrap_or_default(),
            game.event_vm.as_ref().and_then(|vm|vm.pending_battle).map(|(group,_)|group.to_string()).unwrap_or_default(),
            u8::from(game.event_state.switch(0x128))));Ok(())
    };
    let mut back=make()?;back.walk_one_tile(Input {down:true,..Input::default()})?;back.settle_event()?;
    capture("00-backtracking-dialogue",&back)?;
    let mut restored=Game::new();restored.restore_text(&back.save_text())?;
    let before=crate::event_vm::encode_snapshot(&back.event_state,back.event_vm.as_ref());
    if before!=crate::event_vm::encode_snapshot(&restored.event_state,restored.event_vm.as_ref()) {return Err(std::io::Error::other("trigger snapshot differs"));}
    restored.dialogue=None;restored.complete_dialogue(DialogueCompletion::ResumeEvent);restored.settle_event()?;
    capture("01-backtracking-returned",&restored)?;
    let mut guards=make()?;
    for _ in 0..11 {guards.walk_one_tile(Input {up:true,..Input::default()})?;}
    guards.settle_event()?;capture("02-guards-dialogue",&guards)?;
    guards.dialogue=None;guards.complete_dialogue(DialogueCompletion::ResumeEvent);guards.settle_event()?;
    capture("03-guards-battle-boundary",&guards)?;
    let mut copy=Game::new();copy.restore_text(&guards.save_text())?;
    if crate::event_vm::encode_snapshot(&copy.event_state,copy.event_vm.as_ref())!=crate::event_vm::encode_snapshot(&guards.event_state,guards.event_vm.as_ref()) {
        return Err(std::io::Error::other("battle snapshot differs"));
    }
    fs::write(directory.join("states.csv"),summaries)
}

/// Pixel movement, dash, source hooks and camera integration diagnostics.
pub fn capture_player_motion(directory: &std::path::Path) -> std::io::Result<()> {
    fs::create_dir_all(directory)?;
    let mut speed_rows=String::from("cancel,sprint,speed,frame,progress,offset_x,offset_y,completed\n");
    for cancel in [false,true] { for sprint in [false,true] {
        let speed=crate::player_motion::patched_speed(cancel,sprint);
        let mut motion=crate::player_motion::PlayerMotion::default();motion.start(0,speed);
        for frame in 1..=16 {
            let completed=motion.advance().is_some();let (dx,dy)=motion.offset();
            speed_rows.push_str(&format!("{},{},{speed},{frame},{},{dx},{dy},{}\n",u8::from(cancel),u8::from(sprint),motion.progress,u8::from(completed)));
            if completed {break;}
        }
    }}
    fs::write(directory.join("speeds.csv"),speed_rows)?;
    let mut gait=String::from("coordinate,direction,phase\n");
    for coordinate in 0..256 {for direction in 0..4 {
        gait.push_str(&format!("{coordinate},{direction},{}\n",crate::player_motion::patched_walk_phase(coordinate,direction)));
    }}
    fs::write(directory.join("gait.csv"),gait)?;
    let mut rows=String::from("scenario,frame,field_frame,map,x,y,world_x,world_y,steps,active,direction,speed,progress,pending_exit,facing,capture\n");
    for (name,direction,dash,start_y) in [("normal-up",0,false,49),("dash-up",0,true,49),("normal-right",1,false,49),("blocked-right",1,false,49),("exit-up",0,false,2)] {
        let mut game=Game::new();game.scene=Scene::Field;game.opening_phase=3;
        game.x=38;game.y=start_y;game.event_state.actors[0].vehicle=0xc0;
        if name=="normal-right" {
            let map=game.assets.field_maps.get(&19).unwrap();
            let mut found=None;
            for y in 45..61 {for x in 32..44 {
                if map.can_move(x,y,x+1,y) && map.exit_at(x+1,y).is_none() {found=Some((x,y));break;}
            }if found.is_some(){break;}}
            let (x,y)=found.ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound,"no horizontal diagnostic step"))?;
            game.x=x;game.y=y;
        }
        let input=Input {up:direction==0,right:direction==1,dash,..Input::default()};
        let mut canvas=Canvas::new();
        for frame in 0..=64 {
            let (wx,wy)=game.player_world_pixel();
            let capture=if [0,4,8,12,16,20].contains(&frame) {format!("{name}-{frame:02}.bmp")} else {String::new()};
            if !capture.is_empty() {game.draw(&mut canvas);canvas.save_bmp(&directory.join(&capture))?;}
            rows.push_str(&format!("{name},{frame},{},{},{},{},{wx},{wy},{},{},{},{},{},{},{},{}\n",game.field_frame,
                game.map_id,game.x,game.y,game.steps,u8::from(game.motion.active),game.motion.direction,game.motion.speed,
                game.motion.progress,u8::from(game.pending_exit),game.facing,capture));
            if (game.steps!=0 && !game.motion.active && !game.pending_exit) || (name=="blocked-right" && frame==20) {break;}
            game.update(if game.steps==0 {input} else {Input::default()},Duration::from_secs_f64(1.0/60.0));
            if frame==64 {return Err(std::io::Error::new(std::io::ErrorKind::TimedOut,"movement scenario did not complete"));}
        }
    }
    fs::write(directory.join("game-motion.csv"),rows)
}

/// Exercise actual field input and decoded exits. The diagnostic route planner
/// uses static collision only; it does not supply missing original event logic.
pub fn capture_field_route(directory: &std::path::Path) -> std::io::Result<()> {
    let mut game = Game::new();
    game.story_events=false; // Explicit geometry diagnostic; live play dispatches scripts.
    let frame = Duration::from_millis(16);
    for _ in 0..13 {
        game.settle_event()?;
        game.update(Input { confirm: true, ..Input::default() }, frame);
    }
    game.settle_event()?;
    fs::create_dir_all(directory)?;
    let mut canvas = Canvas::new();
    let mut trace = String::from("frame,map,x,y,facing,steps\n");
    let mut steps = String::from("map,x,y,facing,total_steps\n");
    game.draw(&mut canvas);
    canvas.save_bmp(&directory.join("00-map-0019.bmp"))?;
    trace.push_str(&format!("00-map-0019,{},{},{},{},{}\n",
        game.map_id, game.x, game.y, game.facing, game.steps));
    for (stage, destination) in [39, 41, 42, 41].into_iter().enumerate() {
        let map = game.assets.field_maps.get(&game.map_id).ok_or_else(||
            std::io::Error::new(std::io::ErrorKind::NotFound, "Current map bundle missing"))?;
        let route = route_to_exit(map, (game.x, game.y), destination).ok_or_else(||
            std::io::Error::new(std::io::ErrorKind::InvalidData,
                format!("No static collision route from map {} to {destination}", game.map_id)))?;
        for input in route {
            game.walk_one_tile(input)?;
            steps.push_str(&format!("{},{},{},{},{}\n",
                game.map_id, game.x, game.y, game.facing, game.steps));
        }
        if game.map_id != destination {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData,
                format!("Field input did not reach map {destination}")));
        }
        let name = format!("{:02}-map-{:04}", stage + 1, game.map_id);
        game.draw(&mut canvas);
        canvas.save_bmp(&directory.join(format!("{name}.bmp")))?;
        trace.push_str(&format!("{name},{},{},{},{},{}\n",
            game.map_id, game.x, game.y, game.facing, game.steps));
    }
    fs::write(directory.join("state-trace.csv"), trace)?;
    fs::write(directory.join("route-steps.csv"), steps)
}

fn route_to_exit(map: &crate::field::FieldMap, start: (i16, i16), destination: u16) -> Option<Vec<Input>> {
    use std::collections::{HashMap, VecDeque};
    let mut queue = VecDeque::from([start]);
    let mut parents = HashMap::new();
    parents.insert(start, None);
    while let Some(position) = queue.pop_front() {
        if map.exit_at(position.0, position.1)
            .is_some_and(|exit| exit.destination_map() == destination) {
            let mut route = Vec::new();
            let mut current = position;
            while let Some((previous, input)) = parents[&current] {
                route.push(input);
                current = previous;
            }
            route.reverse();
            return Some(route);
        }
        for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
            let next = (position.0 + dx, position.1 + dy);
            if parents.contains_key(&next) || !map.can_move(position.0, position.1, next.0, next.1) {
                continue;
            }
            if map.exit_at(next.0, next.1).is_some_and(|exit| exit.destination_map() != destination) {
                continue;
            }
            let input = Input { up: dy < 0, right: dx > 0, down: dy > 0, left: dx < 0,
                ..Input::default() };
            parents.insert(next, Some((position, input)));
            queue.push_back(next);
        }
    }
    None
}

fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let separator = if line.is_empty() { 0 } else { 1 };
            let needed = word.len() + separator;
            if !line.is_empty() && line.len() + needed > max_chars {
                lines.push(std::mem::take(&mut line));
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::{DialogueCompletion, Game, Input, Scene, wrap_text};
    use std::time::Duration;

    #[test]
    fn new_game_reaches_field_after_intro() {
        let mut game = Game::new();
        game.update(Input { confirm: true, ..Input::default() }, Duration::from_millis(16));
        for _ in 0..5 {
            game.update(Input { confirm: true, ..Input::default() }, Duration::from_millis(16));
        }
        assert_eq!(game.scene, Scene::Field);
    }

    #[test]
    fn target_opening_character_stats_and_guest_save_state() {
        let mut game=Game::new();
        assert_eq!((game.hp,game.max_hp,game.mp,game.max_mp,game.level,game.exp),(63,63,27,27,3,96));
        assert!(game.event_state.switch(0x2f0));
        assert_eq!(game.character_tables.next_level_exp(game.level).unwrap()-game.exp,112);
        assert_eq!(game.characters[14].as_ref().unwrap().property_id,32);
        assert_eq!(game.characters[15].as_ref().unwrap().property_id,33);
        game.hp=17;game.mp=9;game.characters[14].as_mut().unwrap().hp=29;
        let saved=game.save_text();let mut loaded=Game::new();loaded.restore_text(&saved).unwrap();
        assert_eq!((loaded.hp,loaded.mp),(17,9));assert_eq!(loaded.characters[14].as_ref().unwrap().hp,29);
        let missing=saved.lines().filter(|line|!line.starts_with("characters=")).collect::<Vec<_>>().join("\n");
        assert!(loaded.restore_text(&missing).is_err());
        let legacy=missing.replacen("FF6NATIVE9","FF6NATIVE8",1);
        loaded.restore_text(&legacy).unwrap();
        assert_eq!((loaded.hp,loaded.mp),(17,9));
        assert_eq!(loaded.characters[14].as_ref().unwrap().hp,68);
    }

    #[test]
    fn opening_returns_control_after_terras_nine_tile_march() {
        let mut game = Game::new();
        let confirm = Input { confirm: true, ..Input::default() };
        let frame = Duration::from_millis(16);
        game.update(confirm, frame);
        for _ in 0..4 {
            game.update(confirm, frame);
        }
        for _ in 0..7 {
            game.update(confirm, frame);
        }
        game.settle_event().unwrap();
        assert_eq!((game.map_id, game.x, game.y), (19, 38, 50));
        assert_eq!(game.gil, 3000);
        assert!(game.dialogue.is_some());
        game.update(Input { up: true, ..Input::default() }, frame);
        assert_eq!(game.y, 50);
        game.update(confirm, frame);
        game.settle_event().unwrap();
        assert_eq!((game.map_id, game.x, game.y), (19, 38, 49));
        assert_eq!(game.opening_phase, 3);
        assert!(game.dialogue.is_none());
    }

    #[test]
    fn ordinary_messages_cannot_complete_opening_events() {
        let mut game = Game::new();
        game.scene = Scene::Field;
        game.opening_phase = 1;
        game.map_id = 18;
        game.x = 11;
        game.y = 7;
        game.dialogue(&["A NORMAL MESSAGE"]);
        game.update(Input { confirm: true, ..Input::default() }, Duration::from_millis(16));
        assert_eq!((game.opening_phase, game.map_id, game.x, game.y, game.gil),
            (1, 18, 11, 7, 0));
        assert!(game.dialogue.is_none());
    }

    #[test]
    fn walking_across_patched_narshe_exits_round_trips_between_maps() {
        let mut game = Game::new();
        game.scene = Scene::Field;
        game.opening_phase = 3;
        game.map_id = 19;
        game.x = 38;
        game.y = 2;
        game.walk_one_tile(Input { up:true, ..Input::default() }).unwrap();
        assert_eq!((game.map_id, game.x, game.y, game.facing), (39, 26, 52, 0));
        game.walk_one_tile(Input { down:true, ..Input::default() }).unwrap();
        assert_eq!((game.map_id, game.x, game.y, game.facing), (19, 38, 2, 2));
    }

    #[test]
    fn mine_exit_supports_coordinates_above_column_63() {
        let mut game = Game::new();
        game.scene = Scene::Field;
        game.opening_phase = 3;
        game.map_id = 42;
        game.x = 86;
        game.y = 28;
        game.walk_one_tile(Input { down:true, ..Input::default() }).unwrap();
        assert_eq!((game.map_id, game.x, game.y, game.facing), (41, 42, 6, 2));
    }

    #[test]
    fn quicksave_restores_opening_dialogue_and_its_completion_action() {
        let mut game = Game::new();
        let confirm = Input { confirm: true, ..Input::default() };
        let frame = Duration::from_millis(16);
        for _ in 0..8 {
            game.update(confirm, frame);
        }
        assert_eq!(game.dialogue.as_ref().unwrap().current, 3);
        game.event_state.set_switch(0x123, true);
        let saved = game.save_text();
        let mut restored = Game::new();
        restored.restore_text(&saved).unwrap();
        assert!(restored.event_state.switch(0x123));
        assert_eq!(restored.scene, Scene::Field);
        assert_eq!(restored.dialogue.as_ref().unwrap().current, 3);
        assert_eq!(restored.dialogue.as_ref().unwrap().completion, DialogueCompletion::EnterNarshe);
        for _ in 0..4 {
            restored.update(confirm, frame);
        }
        restored.settle_event().unwrap();
        assert_eq!((restored.map_id, restored.y, restored.gil), (19, 50, 3000));
        let saved = restored.save_text();
        restored.restore_text(&saved).unwrap();
        assert_eq!(restored.dialogue.as_ref().unwrap().completion, DialogueCompletion::ResumeEvent);
        assert_eq!(restored.dialogue.as_ref().unwrap().lines[0],
            "Wedge: Let's put her on point.\nNo sense taking any risks.\nForward!");
        restored.update(confirm, frame);
        restored.settle_event().unwrap();
        assert_eq!((restored.opening_phase, restored.y, restored.gil), (3, 49, 3000));
        assert!(restored.dialogue.is_none());
        assert!(restored.event_state.switch(0x1c1));
        assert!(restored.event_state.switch(0x123));
    }

    #[test]
    fn legacy_opening_save_reconstructs_the_vm_without_duplicate_gil() {
        let mut game = Game::new();
        game.restore_text("FF6NATIVE3\nopening_phase=2\nmap_id=19\nx=38\ny=50\ngil=3000\n").unwrap();
        assert!(game.event_vm.is_some());
        assert_eq!(game.gil, 3000);
        game.update(Input { confirm: true, ..Input::default() }, Duration::from_millis(16));
        game.settle_event().unwrap();
        assert_eq!((game.opening_phase, game.y, game.gil), (3, 49, 3000));
        assert!(game.event_vm.is_none());
    }

    #[test]
    fn quicksave_reconstructs_in_flight_actors_before_and_after_dialogue() {
        let mut game = Game::new();
        game.scene = Scene::Field;
        game.start_opening_event(0);
        for target in [25, 100, 150, 200] {
            while game.event_vm.as_ref().unwrap().frame < target { game.advance_event(); }
            let saved = game.save_text();
            let mut restored = Game::new();
            restored.restore_text(&saved).unwrap();
            assert_eq!(restored.event_vm.as_ref().unwrap().frame, game.event_vm.as_ref().unwrap().frame);
            assert_eq!(restored.event_state.actors, game.event_state.actors);
            assert_eq!(restored.gil, game.gil);
            assert_eq!(restored.dialogue.is_some(), game.dialogue.is_some());
        }
        game.settle_event().unwrap();
        game.dialogue = None;
        for _ in 0..45 { game.advance_event(); }
        assert!(game.dialogue.is_none());
        let mut restored = Game::new();
        restored.restore_text(&game.save_text()).unwrap();
        assert_eq!(restored.event_state.actors, game.event_state.actors);
        game.settle_event().unwrap();
        restored.settle_event().unwrap();
        assert_eq!((restored.y, restored.gil, restored.opening_phase), (game.y, game.gil, game.opening_phase));
    }

    #[test]
    fn player_moves_pixels_locks_step_speed_and_defers_menu_until_alignment() {
        let mut game = Game::new(); game.scene=Scene::Field; game.opening_phase=3;
        game.x=38; game.y=49;
        let tick=Duration::from_secs_f64(1.0/60.0);
        for _ in 0..4 { game.update(Input { up:true,..Input::default() },tick); }
        assert_eq!((game.x,game.y,game.steps,game.motion.progress),(38,49,1,256));
        assert_eq!(game.player_world_pixel(),(608,783));
        game.update(Input { menu:true,right:true,dash:true,..Input::default() },tick);
        assert_eq!((game.scene,game.facing,game.motion.speed),(Scene::Field,0,2));
        for _ in 0..14 { game.update(Input { right:true,dash:true,..Input::default() },tick); }
        assert_eq!((game.x,game.y,game.steps),(38,48,1));
        assert!(!game.motion.active);
        assert_eq!(game.scene,Scene::Field);
        game.update(Input::default(),tick);
        assert_eq!(game.scene,Scene::Menu);
    }

    #[test]
    fn sprint_shoes_use_enabled_party_members_and_combine_with_cancel_dash() {
        for dash in [false,true] {
            let mut game=Game::new();game.scene=Scene::Field;game.opening_phase=3;game.x=38;game.y=49;
            game.characters[14].as_mut().unwrap().properties[19]=230;
            game.event_state.char_parties[14]=1;
            assert_eq!(game.party_field_effects()&0x20,0);
            game.event_state.actors[14].enabled=true;
            assert_eq!(game.party_field_effects()&0x20,0x20);
            game.event_state.char_parties[14]=2;assert_eq!(game.party_field_effects()&0x20,0);
            game.event_state.char_parties[14]=1;
            let tick=Duration::from_secs_f64(1.0/60.0);
            for _ in 0..4 {game.update(Input {up:true,dash,..Input::default()},tick);}
            assert_eq!(game.motion.speed,if dash {4} else {3});
            assert_eq!(game.motion.progress,if dash {1024} else {512});
            let mut restored=Game::new();restored.restore_text(&game.save_text()).unwrap();
            assert_eq!(restored.party_field_effects()&0x20,0x20);
            assert_eq!(restored.motion,game.motion);
        }
    }

    #[test]
    fn entrance_waits_until_the_frame_after_pixel_motion_finishes() {
        let mut game=Game::new();game.scene=Scene::Field;game.opening_phase=3;
        game.x=38;game.y=2;
        let tick=Duration::from_secs_f64(1.0/60.0);
        for _ in 0..18 {game.update(Input {up:true,..Input::default()},tick);}
        assert_eq!((game.map_id,game.y,game.motion.progress),(19,2,3840));
        game.update(Input::default(),tick);
        assert_eq!((game.map_id,game.y),(19,1));assert!(game.pending_exit);
        game.update(Input::default(),tick);
        assert_eq!((game.map_id,game.x,game.y),(39,26,52));
    }

    #[test]
    fn field_save_restores_subtile_position_clock_and_locked_velocity() {
        let mut game=Game::new();game.scene=Scene::Field;game.opening_phase=3;
        game.x=38;game.y=49;game.event_state.actors[0].vehicle=0xc0;
        let tick=Duration::from_secs_f64(1.0/60.0);
        for _ in 0..7 {game.update(Input {up:true,dash:true,..Input::default()},tick);}
        assert_eq!(game.motion.progress,2048);
        let mut restored=Game::new();restored.restore_text(&game.save_text()).unwrap();
        restored.dialogue=None;
        assert_eq!(restored.motion,game.motion);
        assert_eq!(restored.player_world_pixel(),game.player_world_pixel());
        assert_eq!(restored.field_frame,game.field_frame);
        assert_eq!(restored.field_clock,game.field_clock);
        for _ in 0..8 {game.update(Input::default(),tick);restored.update(Input::default(),tick);}
        assert_eq!((restored.x,restored.y,restored.steps),(game.x,game.y,game.steps));
        assert_eq!(restored.motion,game.motion);
        let bad=game.save_text().replace("field_motion=0,3,0,0","field_motion=0,3,256,1");
        assert!(restored.restore_text(&bad).is_err());
    }

    fn completed_opening()->Game {
        let mut game=Game::new();game.scene=Scene::Field;game.map_id=18;game.start_opening_event(0);
        game.settle_event().unwrap();game.complete_dialogue(DialogueCompletion::ResumeEvent);game.dialogue=None;
        game.settle_event().unwrap();game
    }

    #[test]
    fn walking_back_to_opening_trigger_runs_and_restores_target_scene() {
        let mut game=completed_opening();game.walk_one_tile(Input {down:true,..Input::default()}).unwrap();
        game.settle_event().unwrap();assert_eq!(game.event_dialogue,Some((12,0)));
        assert_eq!(game.event_state.party,[0,255,255,255]);
        assert_eq!((game.event_state.char_parties[0],game.event_state.char_parties[14],game.event_state.char_parties[15]),(1,1,1));
        assert_eq!((game.event_state.actors[14].palette,game.event_state.actors[15].palette),(1,1));
        assert_eq!((game.event_state.actors[14].x,game.event_state.actors[14].y),(38,52));
        let mut restored=Game::new();restored.restore_text(&game.save_text()).unwrap();
        assert_eq!(restored.event_dialogue,game.event_dialogue);assert_eq!(restored.event_state.actors,game.event_state.actors);
        for current in [&mut game,&mut restored] {
            current.update(Input {confirm:true,..Input::default()},Duration::from_millis(17));current.settle_event().unwrap();
            assert_eq!((current.map_id,current.x,current.y,current.gil),(19,38,49,3000));
            assert!(!current.event_state.actors[14].visible);assert!(!current.event_state.actors[15].visible);
            assert!(current.event_vm.is_none());
        }
    }

    #[test]
    fn first_guard_trigger_uses_target_npcs_and_stops_at_real_battle_request() {
        let mut game=completed_opening();
        for _ in 0..11 {game.walk_one_tile(Input {up:true,..Input::default()}).unwrap();}
        game.settle_event().unwrap();assert_eq!(game.event_dialogue,Some((13,0x8000)));
        assert_eq!((game.event_state.actors[16].graphic,game.event_state.actors[16].palette,game.event_state.actors[16].y),(52,1,35));
        assert!(game.event_state.actors[16].enabled && game.event_state.actors[17].visible);
        game.update(Input {confirm:true,..Input::default()},Duration::from_millis(17));game.settle_event().unwrap();
        assert!(game.event_fault);assert_eq!(game.event_vm.as_ref().unwrap().pending_battle,Some((1,0x3f)));
        assert!(!game.event_state.switch(0x128));
        let mut restored=Game::new();restored.restore_text(&game.save_text()).unwrap();
        assert!(restored.event_fault);assert_eq!(restored.event_vm.as_ref().unwrap().pending_battle,Some((1,0x3f)));
        assert_eq!(restored.event_state.actors,game.event_state.actors);
        game.update(Input {confirm:true,..Input::default()},Duration::from_millis(17));
        let pc=game.event_vm.as_ref().unwrap().pc;
        for _ in 0..20 {game.update(Input {up:true,..Input::default()},Duration::from_millis(17));}
        assert_eq!(game.event_vm.as_ref().unwrap().pc,pc);assert_eq!(game.y,38);
    }

    #[test]
    fn completed_guard_bit_branches_to_target_event_return_without_blocking_movement() {
        let mut game=completed_opening();game.y=38;game.event_state.set_switch(0x128,true);
        game.walk_one_tile(Input {up:true,..Input::default()}).unwrap();
        assert_eq!(game.y,37);assert!(game.event_vm.is_none());assert!(game.dialogue.is_none());
        assert!(!game.event_state.actors[16].visible);
    }

    #[test]
    fn vehicle_flags_survive_control_return_save_and_legacy_restore() {
        let mut game = Game::new();
        game.scene = Scene::Field;
        game.start_opening_event(0);
        game.settle_event().unwrap();
        game.dialogue = None;
        game.settle_event().unwrap();
        assert_eq!(game.opening_phase, 3);
        assert_eq!(game.event_state.actors[0].vehicle, 0xc0);
        let mut restored = Game::new();
        restored.restore_text(&game.save_text()).unwrap();
        assert_eq!(restored.event_state.actors[0].vehicle, 0xc0);
        restored.restore_text("FF6NATIVE5\nopening_phase=3\nmap_id=19\nx=38\ny=49\n").unwrap();
        assert_eq!(restored.event_state.actors[0].vehicle, 0xc0);
    }

    #[test]
    fn unsupported_event_remains_halted_after_dismissing_the_error() {
        let mut game = Game::new();
        game.scene = Scene::Field;
        game.assets.event_script[crate::event_vm::NARSHE_ENTRY] = 0xf9;
        game.start_opening_event(0);
        assert!(game.event_fault);
        let pc = game.event_vm.as_ref().unwrap().pc;
        game.update(Input { confirm: true, ..Input::default() }, Duration::from_millis(17));
        for _ in 0..60 { game.update(Input { up: true, ..Input::default() }, Duration::from_millis(17)); }
        assert_eq!(game.event_vm.as_ref().unwrap().pc, pc);
        assert_eq!(game.gil, 0);
    }

    #[test]
    fn wrapping_preserves_explicit_dialogue_rows_and_blank_lines() {
        assert_eq!(wrap_text("ONE TWO\n\nTHREE FOUR FIVE", 10),
            vec!["ONE TWO", "", "THREE FOUR", "FIVE"]);
    }
}
