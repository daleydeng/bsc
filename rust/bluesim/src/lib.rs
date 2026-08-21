//! Versioned SimIR loading and the deliberately small Bluesim interpreter.
//!
//! Schema version 1 is intentionally limited to the `tiny.bsv` vertical slice.
//! Schema version 2 adds a closed, event-ordered two-clock/initial-reset slice.
//! Schema version 3 adds a closed, flattened hierarchy/method slice on one clock.
//! Unsupported semantics must require a schema/runtime extension; they must never
//! be guessed or silently delegated to legacy Bluesim.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde::Deserialize;
use thiserror::Error;

pub const SIMIR_SCHEMA_VERSION: u32 = 1;
pub const SIMIR_M2_SCHEMA_VERSION: u32 = 2;
pub const SIMIR_M3_SCHEMA_VERSION: u32 = 3;
pub const SIMIR_M0_SCHEMA_VERSION: u32 = SIMIR_SCHEMA_VERSION;

#[derive(Debug, Error)]
pub enum Error {
    #[error("read SimIR {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("decode SimIR: {0}")]
    Decode(#[from] serde_json::Error),
    #[error("invalid SimIR: {0}")]
    Validation(String),
    #[error("simulation did not finish within {max_cycles} cycles")]
    CycleLimit { max_cycles: u64 },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Model {
    pub schema_version: u32,
    pub producer: Producer,
    pub top: String,
    pub clocks: Vec<Clock>,
    pub state: Vec<StateCell>,
    pub schedules: Vec<Schedule>,
    #[serde(default)]
    pub resets: Vec<InitialReset>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Producer {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Clock {
    pub id: String,
    pub period: u64,
    pub active_edge: ActiveEdge,
    #[serde(default)]
    pub order: Option<u32>,
    #[serde(default)]
    pub initial_value: Option<ClockValue>,
    #[serde(default)]
    pub first_edge: Option<u64>,
    #[serde(default)]
    pub high_duration: Option<u64>,
    #[serde(default)]
    pub low_duration: Option<u64>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActiveEdge {
    Posedge,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClockValue {
    Low,
    High,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InitialReset {
    pub id: String,
    pub signal: String,
    pub clock: String,
    pub cycles: u64,
    pub targets: Vec<ResetTarget>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResetTarget {
    pub state: String,
    pub value: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StateCell {
    pub id: String,
    pub width: u8,
    pub initial_value: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Schedule {
    pub clock: String,
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    If {
        condition: Expr,
        #[serde(rename = "then")]
        then_actions: Vec<Action>,
        #[serde(rename = "else")]
        else_actions: Vec<Action>,
    },
    Let {
        local: String,
        value: Expr,
    },
    Write {
        state: String,
        value: Expr,
    },
    Display {
        items: Vec<DisplayItem>,
    },
    Finish {
        status: i32,
    },
    ResetTick {
        reset: String,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DisplayItem {
    Text { text: String },
    Time { width: u16 },
    Decimal { width: u16, value: Expr },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Expr {
    State {
        id: String,
    },
    Local {
        id: String,
    },
    Time,
    Const {
        width: u8,
        value: u64,
    },
    Unary {
        width: u8,
        op: UnaryOp,
        arg: Box<Expr>,
    },
    Binary {
        width: u8,
        op: BinaryOp,
        args: Vec<Expr>,
    },
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnaryOp {
    Not,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BinaryOp {
    Add,
    And,
    Equal,
    UnsignedLessThan,
    Sub,
}

impl Model {
    pub fn read_json(path: &Path) -> Result<Self, Error> {
        let source = fs::read_to_string(path).map_err(|source| Error::Read {
            path: path.display().to_string(),
            source,
        })?;
        Self::from_json(&source)
    }

    pub fn from_json(source: &str) -> Result<Self, Error> {
        let model: Self = serde_json::from_str(source)?;
        model.validate()?;
        Ok(model)
    }

    pub fn validate(&self) -> Result<(), Error> {
        if !matches!(
            self.schema_version,
            SIMIR_M0_SCHEMA_VERSION | SIMIR_M2_SCHEMA_VERSION | SIMIR_M3_SCHEMA_VERSION
        ) {
            return invalid(format!(
                "unsupported schema version {}; expected {SIMIR_M0_SCHEMA_VERSION}, {SIMIR_M2_SCHEMA_VERSION}, or {SIMIR_M3_SCHEMA_VERSION}",
                self.schema_version
            ));
        }
        nonempty(&self.producer.name, "producer name")?;
        nonempty(&self.producer.version, "producer version")?;
        nonempty(&self.top, "top")?;
        if self.clocks.is_empty() {
            return invalid("model has no clocks");
        }
        if self.schedules.is_empty() {
            return invalid("model has no schedules");
        }

        let mut clocks = BTreeSet::new();
        for clock in &self.clocks {
            nonempty(&clock.id, "clock id")?;
            if !clocks.insert(clock.id.as_str()) {
                return invalid(format!("duplicate clock id {:?}", clock.id));
            }
            if clock.period == 0 {
                return invalid(format!("clock {:?} has a zero period", clock.id));
            }
            validate_clock_version(self.schema_version, clock)?;
        }

        let mut state = BTreeMap::new();
        for cell in &self.state {
            nonempty(&cell.id, "state id")?;
            if cell.width == 0 || cell.width > 64 {
                return invalid(format!(
                    "state {:?} has unsupported width {}; M0 supports 1..=64",
                    cell.id, cell.width
                ));
            }
            if cell.initial_value > value_mask(cell.width) {
                return invalid(format!(
                    "state {:?} initial value does not fit its {}-bit width",
                    cell.id, cell.width
                ));
            }
            if state.insert(cell.id.clone(), cell.width).is_some() {
                return invalid(format!("duplicate state id {:?}", cell.id));
            }
        }

        let resets = validate_resets(self.schema_version, &self.resets, &clocks, &state)?;
        let mut scheduled_clocks = BTreeSet::new();
        for schedule in &self.schedules {
            if !clocks.contains(schedule.clock.as_str()) {
                return invalid(format!(
                    "schedule references unknown clock {:?}",
                    schedule.clock
                ));
            }
            if !scheduled_clocks.insert(schedule.clock.as_str()) {
                return invalid(format!("multiple schedules for clock {:?}", schedule.clock));
            }
            validate_actions(&schedule.actions, &state, &mut BTreeMap::new())?;
            validate_reset_ticks(self.schema_version, schedule, &resets, &state)?;
        }
        if self.schema_version == SIMIR_M2_SCHEMA_VERSION && scheduled_clocks.len() != clocks.len()
        {
            return invalid("M2 requires exactly one schedule for every clock");
        }
        Ok(())
    }
}

fn validate_clock_version(version: u32, clock: &Clock) -> Result<(), Error> {
    let waveform = (
        clock.order,
        clock.initial_value,
        clock.first_edge,
        clock.high_duration,
        clock.low_duration,
    );
    if version != SIMIR_M2_SCHEMA_VERSION {
        if waveform != (None, None, None, None, None) {
            return invalid("single-clock SimIR contains M2 waveform fields");
        }
        return Ok(());
    }
    let (Some(_), Some(_), Some(_), Some(high), Some(low)) = waveform else {
        return invalid(format!(
            "M2 clock {:?} is missing waveform fields",
            clock.id
        ));
    };
    if high == 0 || low == 0 || clock.period != high.saturating_add(low) {
        return invalid(format!(
            "M2 clock {:?} has inconsistent period/high/low durations",
            clock.id
        ));
    }
    Ok(())
}

fn validate_resets<'a>(
    version: u32,
    resets: &'a [InitialReset],
    clocks: &BTreeSet<&str>,
    state: &BTreeMap<String, u8>,
) -> Result<BTreeMap<String, &'a InitialReset>, Error> {
    if version != SIMIR_M2_SCHEMA_VERSION && !resets.is_empty() {
        return invalid("single-clock SimIR model contains M2 resets");
    }
    let mut ids = BTreeMap::new();
    for reset in resets {
        nonempty(&reset.id, "reset id")?;
        nonempty(&reset.signal, "reset signal")?;
        if !clocks.contains(reset.clock.as_str()) {
            return invalid(format!("reset {:?} references unknown clock", reset.id));
        }
        if reset.cycles == 0 {
            return invalid(format!("reset {:?} has zero cycles", reset.id));
        }
        if state.get(&reset.signal) != Some(&1) {
            return invalid(format!(
                "reset {:?} signal {:?} must be a 1-bit state",
                reset.id, reset.signal
            ));
        }
        if ids.insert(reset.id.clone(), reset).is_some() {
            return invalid(format!("duplicate reset id {:?}", reset.id));
        }
        let mut targets = BTreeSet::new();
        for target in &reset.targets {
            let Some(width) = state.get(&target.state) else {
                return invalid(format!(
                    "reset {:?} references unknown target state {:?}",
                    reset.id, target.state
                ));
            };
            if target.value > value_mask(*width) {
                return invalid(format!(
                    "reset {:?} value does not fit target state {:?}",
                    reset.id, target.state
                ));
            }
            if !targets.insert(target.state.as_str()) {
                return invalid(format!(
                    "reset {:?} has duplicate target state {:?}",
                    reset.id, target.state
                ));
            }
        }
    }
    Ok(ids)
}

fn validate_reset_ticks(
    version: u32,
    schedule: &Schedule,
    resets: &BTreeMap<String, &InitialReset>,
    state: &BTreeMap<String, u8>,
) -> Result<(), Error> {
    let ticks = schedule
        .actions
        .iter()
        .filter_map(|action| match action {
            Action::ResetTick { reset } => Some(reset),
            _ => None,
        })
        .collect::<Vec<_>>();
    if version != SIMIR_M2_SCHEMA_VERSION && !ticks.is_empty() {
        return invalid("single-clock SimIR schedule contains M2 reset tick");
    }
    for reset in ticks {
        let Some(definition) = resets.get(reset.as_str()) else {
            return invalid(format!("reset tick references unknown reset {reset:?}"));
        };
        if definition.clock != schedule.clock {
            return invalid(format!(
                "reset tick {:?} is not on its clock {:?}",
                reset, definition.clock
            ));
        }
        if schedule.actions.last().is_none_or(
            |action| !matches!(action, Action::ResetTick { reset: last } if last == reset),
        ) {
            return invalid(format!(
                "reset tick {:?} must be the final schedule action",
                reset
            ));
        }
        if state.get(&definition.signal) != Some(&1) {
            return invalid(format!("reset {:?} signal has invalid width", reset));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunResult {
    pub output: Vec<String>,
    pub exit_status: Option<i32>,
    pub cycles: u64,
    pub time: u64,
}

pub struct Engine {
    model: Model,
    state: BTreeMap<String, u64>,
    next_edges: BTreeMap<String, u64>,
    reset_remaining: BTreeMap<String, u64>,
    time: u64,
    cycles: u64,
    exit_status: Option<i32>,
}

impl Engine {
    pub fn new(model: Model) -> Result<Self, Error> {
        model.validate()?;
        Ok(Self {
            state: model
                .state
                .iter()
                .map(|cell| (cell.id.clone(), cell.initial_value))
                .collect(),
            next_edges: model
                .clocks
                .iter()
                .filter_map(|clock| {
                    (model.schema_version == SIMIR_M2_SCHEMA_VERSION).then(|| {
                        (
                            clock.id.clone(),
                            initial_posedge(clock).expect("validated M2 clock"),
                        )
                    })
                })
                .collect(),
            reset_remaining: model
                .resets
                .iter()
                .map(|reset| (reset.id.clone(), reset.cycles))
                .collect(),
            model,
            time: 0,
            cycles: 0,
            exit_status: None,
        })
    }

    pub fn step(&mut self, cycles: u64) -> Result<RunResult, Error> {
        let mut output = Vec::new();
        for _ in 0..cycles {
            if self.exit_status.is_some() {
                break;
            }
            if self.model.schema_version != SIMIR_M2_SCHEMA_VERSION {
                self.step_once_m0(&mut output)?;
            } else {
                self.step_once_m2(&mut output)?;
            }
        }
        Ok(self.result(output))
    }

    pub fn run(&mut self, max_cycles: u64) -> Result<RunResult, Error> {
        let result = self.step(max_cycles)?;
        if result.exit_status.is_none() {
            return Err(Error::CycleLimit { max_cycles });
        }
        Ok(result)
    }

    fn step_once_m0(&mut self, output: &mut Vec<String>) -> Result<(), Error> {
        let snapshot = self.state.clone();
        let mut writes = BTreeMap::new();
        for schedule in &self.model.schedules {
            let clock = self
                .model
                .clocks
                .iter()
                .find(|clock| clock.id == schedule.clock)
                .expect("validated schedule clock");
            self.time = self
                .time
                .max(self.cycles.saturating_add(1).saturating_mul(clock.period));
            execute_actions(
                &schedule.actions,
                &snapshot,
                &mut BTreeMap::new(),
                self.time,
                &mut writes,
                output,
                &mut self.exit_status,
            );
            if self.exit_status.is_some() {
                break;
            }
        }
        for (id, value) in writes {
            let width = self
                .model
                .state
                .iter()
                .find(|cell| cell.id == id)
                .expect("validated write state")
                .width;
            self.state.insert(id, value & value_mask(width));
        }
        self.cycles += 1;
        Ok(())
    }

    fn step_once_m2(&mut self, output: &mut Vec<String>) -> Result<(), Error> {
        let (clock_id, time) = self
            .next_edges
            .iter()
            .min_by_key(|(id, time)| {
                let order = self
                    .model
                    .clocks
                    .iter()
                    .find(|clock| &clock.id == *id)
                    .expect("validated M2 clock")
                    .order
                    .expect("validated M2 clock order");
                (**time, order)
            })
            .map(|(id, time)| (id.clone(), *time))
            .expect("validated M2 model has clocks");
        let schedule = self
            .model
            .schedules
            .iter()
            .find(|schedule| schedule.clock == clock_id)
            .expect("validated M2 schedule")
            .clone();
        let clock = self
            .model
            .clocks
            .iter()
            .find(|clock| clock.id == clock_id)
            .expect("validated M2 clock");
        self.time = time;
        let snapshot = self.state.clone();
        let mut writes = BTreeMap::new();
        execute_actions_m2(
            &schedule.actions,
            &snapshot,
            &mut BTreeMap::new(),
            self.time,
            &mut writes,
            output,
            &mut self.exit_status,
            &clock_id,
            &self.model.resets,
            &mut self.reset_remaining,
        );
        for (id, value) in writes {
            let width = self
                .model
                .state
                .iter()
                .find(|cell| cell.id == id)
                .expect("validated write state")
                .width;
            self.state.insert(id, value & value_mask(width));
        }
        self.next_edges
            .insert(clock_id, time.saturating_add(clock.period));
        self.cycles += 1;
        Ok(())
    }

    fn result(&self, output: Vec<String>) -> RunResult {
        RunResult {
            output,
            exit_status: self.exit_status,
            cycles: self.cycles,
            time: self.time,
        }
    }
}

fn initial_posedge(clock: &Clock) -> Option<u64> {
    match (clock.initial_value, clock.first_edge, clock.low_duration) {
        (Some(ClockValue::Low), Some(first_edge), _) => Some(first_edge),
        (Some(ClockValue::High), Some(first_edge), Some(low_duration)) => {
            Some(first_edge.saturating_add(low_duration))
        }
        _ => None,
    }
}

fn execute_actions(
    actions: &[Action],
    state: &BTreeMap<String, u64>,
    locals: &mut BTreeMap<String, u64>,
    time: u64,
    writes: &mut BTreeMap<String, u64>,
    output: &mut Vec<String>,
    exit_status: &mut Option<i32>,
) {
    for action in actions {
        if exit_status.is_some() {
            return;
        }
        match action {
            Action::If {
                condition,
                then_actions,
                else_actions,
            } => {
                let selected = if eval(condition, state, locals, time) != 0 {
                    then_actions
                } else {
                    else_actions
                };
                let mut branch_locals = locals.clone();
                execute_actions(
                    selected,
                    state,
                    &mut branch_locals,
                    time,
                    writes,
                    output,
                    exit_status,
                );
            }
            Action::Let { local, value } => {
                locals.insert(local.clone(), eval(value, state, locals, time));
            }
            Action::Write { state: id, value } => {
                writes.insert(id.clone(), eval(value, state, locals, time));
            }
            Action::Display { items } => {
                let mut line = String::new();
                for item in items {
                    match item {
                        DisplayItem::Text { text } => line.push_str(text),
                        DisplayItem::Time { width } => {
                            line.push_str(&format!("{:>width$}", time, width = *width as usize));
                        }
                        DisplayItem::Decimal { width, value } => {
                            line.push_str(&format!(
                                "{:>width$}",
                                eval(value, state, locals, time),
                                width = *width as usize
                            ));
                        }
                    }
                }
                output.push(line);
            }
            Action::Finish { status } => *exit_status = Some(*status),
            Action::ResetTick { .. } => unreachable!("M0 validation rejects reset ticks"),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_actions_m2(
    actions: &[Action],
    state: &BTreeMap<String, u64>,
    locals: &mut BTreeMap<String, u64>,
    time: u64,
    writes: &mut BTreeMap<String, u64>,
    output: &mut Vec<String>,
    exit_status: &mut Option<i32>,
    clock: &str,
    resets: &[InitialReset],
    reset_remaining: &mut BTreeMap<String, u64>,
) {
    for action in actions {
        if exit_status.is_some() {
            return;
        }
        match action {
            Action::If {
                condition,
                then_actions,
                else_actions,
            } => {
                let selected = if eval(condition, state, locals, time) != 0 {
                    then_actions
                } else {
                    else_actions
                };
                let mut branch_locals = locals.clone();
                execute_actions_m2(
                    selected,
                    state,
                    &mut branch_locals,
                    time,
                    writes,
                    output,
                    exit_status,
                    clock,
                    resets,
                    reset_remaining,
                );
            }
            Action::Let { local, value } => {
                locals.insert(local.clone(), eval(value, state, locals, time));
            }
            Action::Write { state: id, value } => {
                writes.insert(id.clone(), eval(value, state, locals, time));
            }
            Action::Display { items } => {
                let mut line = String::new();
                for item in items {
                    match item {
                        DisplayItem::Text { text } => line.push_str(text),
                        DisplayItem::Time { width } => {
                            line.push_str(&format!("{:>width$}", time, width = *width as usize));
                        }
                        DisplayItem::Decimal { width, value } => {
                            line.push_str(&format!(
                                "{:>width$}",
                                eval(value, state, locals, time),
                                width = *width as usize
                            ));
                        }
                    }
                }
                output.push(line);
            }
            Action::Finish { status } => *exit_status = Some(*status),
            Action::ResetTick { reset } => {
                let definition = resets
                    .iter()
                    .find(|definition| definition.id == *reset)
                    .expect("validated M2 reset tick");
                debug_assert_eq!(definition.clock, clock);
                if state[&definition.signal] == 0 {
                    for target in &definition.targets {
                        writes.insert(target.state.clone(), target.value);
                    }
                    let remaining = reset_remaining
                        .get_mut(reset)
                        .expect("validated M2 reset state");
                    if *remaining == 1 {
                        writes.insert(definition.signal.clone(), 1);
                    }
                    *remaining = remaining.saturating_sub(1);
                }
            }
        }
    }
}

fn eval(
    expr: &Expr,
    state: &BTreeMap<String, u64>,
    locals: &BTreeMap<String, u64>,
    time: u64,
) -> u64 {
    match expr {
        Expr::State { id } => state[id],
        Expr::Local { id } => locals[id],
        Expr::Time => time,
        Expr::Const { value, .. } => *value,
        Expr::Unary { width, op, arg } => {
            let value = match op {
                UnaryOp::Not => !eval(arg, state, locals, time),
            };
            value & value_mask(*width)
        }
        Expr::Binary { width, op, args } => {
            let left = eval(&args[0], state, locals, time);
            let right = eval(&args[1], state, locals, time);
            let value = match op {
                BinaryOp::Add => left.wrapping_add(right),
                BinaryOp::And => left & right,
                BinaryOp::Equal => u64::from(left == right),
                BinaryOp::UnsignedLessThan => u64::from(left < right),
                BinaryOp::Sub => left.wrapping_sub(right),
            };
            value & value_mask(*width)
        }
    }
}

fn validate_actions(
    actions: &[Action],
    state: &BTreeMap<String, u8>,
    locals: &mut BTreeMap<String, u8>,
) -> Result<(), Error> {
    for action in actions {
        match action {
            Action::If {
                condition,
                then_actions,
                else_actions,
            } => {
                if validate_expr(condition, state, locals)? != 1 {
                    return invalid("if condition must have width 1");
                }
                validate_actions(then_actions, state, &mut locals.clone())?;
                validate_actions(else_actions, state, &mut locals.clone())?;
            }
            Action::Let { local, value } => {
                nonempty(local, "local id")?;
                if locals.contains_key(local) {
                    return invalid(format!("duplicate local id {local:?}"));
                }
                let width = validate_expr(value, state, locals)?;
                locals.insert(local.clone(), width);
            }
            Action::Write { state: id, value } => {
                let expected = state.get(id).ok_or_else(|| {
                    Error::Validation(format!("write references unknown state {id:?}"))
                })?;
                let actual = validate_expr(value, state, locals)?;
                if actual != *expected {
                    return invalid(format!(
                        "write to state {id:?} has {actual}-bit value; expected {expected}-bit value"
                    ));
                }
            }
            Action::Display { items } => {
                for item in items {
                    if let DisplayItem::Decimal { value, .. } = item {
                        validate_expr(value, state, locals)?;
                    }
                }
            }
            Action::Finish { .. } | Action::ResetTick { .. } => {}
        }
    }
    Ok(())
}

fn validate_expr(
    expr: &Expr,
    state: &BTreeMap<String, u8>,
    locals: &BTreeMap<String, u8>,
) -> Result<u8, Error> {
    match expr {
        Expr::State { id } => state.get(id).copied().ok_or_else(|| {
            Error::Validation(format!("expression references unknown state {id:?}"))
        }),
        Expr::Local { id } => locals.get(id).copied().ok_or_else(|| {
            Error::Validation(format!("expression references unknown local {id:?}"))
        }),
        Expr::Time => Ok(64),
        Expr::Const { width, value } => {
            validate_width(*width, "constant")?;
            if *value > value_mask(*width) {
                return invalid(format!("constant value does not fit its {width}-bit width"));
            }
            Ok(*width)
        }
        Expr::Unary { width, op: _, arg } => {
            validate_width(*width, "unary expression")?;
            let argument_width = validate_expr(arg, state, locals)?;
            if *width != argument_width {
                return invalid("unary expression and its argument must have the same width");
            }
            Ok(*width)
        }
        Expr::Binary { width, op, args } => {
            validate_width(*width, "binary expression")?;
            if args.len() != 2 {
                return invalid("binary expression must have exactly two arguments");
            }
            let left = validate_expr(&args[0], state, locals)?;
            let right = validate_expr(&args[1], state, locals)?;
            match op {
                BinaryOp::Add | BinaryOp::And | BinaryOp::Sub
                    if left == right && *width == left =>
                {
                    Ok(*width)
                }
                BinaryOp::Equal | BinaryOp::UnsignedLessThan if left == right && *width == 1 => {
                    Ok(1)
                }
                BinaryOp::Add | BinaryOp::And | BinaryOp::Sub => {
                    invalid("bitwise/arithmetic operands and result must have the same width")
                }
                BinaryOp::Equal | BinaryOp::UnsignedLessThan => {
                    invalid("comparison operands must have equal widths and a 1-bit result")
                }
            }
        }
    }
}

fn validate_width(width: u8, subject: &str) -> Result<(), Error> {
    if (1..=64).contains(&width) {
        Ok(())
    } else {
        invalid(format!(
            "{subject} has unsupported width {width}; M0 supports 1..=64"
        ))
    }
}

fn value_mask(width: u8) -> u64 {
    if width == 64 {
        u64::MAX
    } else {
        (1_u64 << width) - 1
    }
}

fn nonempty(value: &str, subject: &str) -> Result<(), Error> {
    if value.is_empty() {
        invalid(format!("{subject} must not be empty"))
    } else {
        Ok(())
    }
}

fn invalid<T>(message: impl Into<String>) -> Result<T, Error> {
    Err(Error::Validation(message.into()))
}
