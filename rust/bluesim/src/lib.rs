//! Versioned SimIR loading and the deliberately small M0 Bluesim interpreter.
//!
//! Schema version 1 is intentionally limited to the `tiny.bsv` vertical slice:
//! 64-bit-or-narrow unsigned values, periodic positive-edge clocks, state reads,
//! add/equality/unsigned-less-than expressions, conditional actions, writes,
//! `$display`, and `$finish`.  Unsupported semantics must require a schema/runtime
//! extension; they must never be guessed or silently delegated to legacy Bluesim.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde::Deserialize;
use thiserror::Error;

pub const SIMIR_SCHEMA_VERSION: u32 = 1;

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
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActiveEdge {
    Posedge,
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
    Const {
        width: u8,
        value: u64,
    },
    Binary {
        width: u8,
        op: BinaryOp,
        args: Vec<Expr>,
    },
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BinaryOp {
    Add,
    Equal,
    UnsignedLessThan,
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
        if self.schema_version != SIMIR_SCHEMA_VERSION {
            return invalid(format!(
                "unsupported schema version {}; expected {SIMIR_SCHEMA_VERSION}",
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
            if state.insert(cell.id.as_str(), cell.width).is_some() {
                return invalid(format!("duplicate state id {:?}", cell.id));
            }
        }

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
            validate_actions(&schedule.actions, &state)?;
        }
        Ok(())
    }
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
            self.step_once(&mut output)?;
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

    fn step_once(&mut self, output: &mut Vec<String>) -> Result<(), Error> {
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

    fn result(&self, output: Vec<String>) -> RunResult {
        RunResult {
            output,
            exit_status: self.exit_status,
            cycles: self.cycles,
            time: self.time,
        }
    }
}

fn execute_actions(
    actions: &[Action],
    state: &BTreeMap<String, u64>,
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
                let selected = if eval(condition, state) != 0 {
                    then_actions
                } else {
                    else_actions
                };
                execute_actions(selected, state, time, writes, output, exit_status);
            }
            Action::Write { state: id, value } => {
                writes.insert(id.clone(), eval(value, state));
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
                                eval(value, state),
                                width = *width as usize
                            ));
                        }
                    }
                }
                output.push(line);
            }
            Action::Finish { status } => *exit_status = Some(*status),
        }
    }
}

fn eval(expr: &Expr, state: &BTreeMap<String, u64>) -> u64 {
    match expr {
        Expr::State { id } => state[id],
        Expr::Const { value, .. } => *value,
        Expr::Binary { width, op, args } => {
            let left = eval(&args[0], state);
            let right = eval(&args[1], state);
            let value = match op {
                BinaryOp::Add => left.wrapping_add(right),
                BinaryOp::Equal => u64::from(left == right),
                BinaryOp::UnsignedLessThan => u64::from(left < right),
            };
            value & value_mask(*width)
        }
    }
}

fn validate_actions(actions: &[Action], state: &BTreeMap<&str, u8>) -> Result<(), Error> {
    for action in actions {
        match action {
            Action::If {
                condition,
                then_actions,
                else_actions,
            } => {
                if validate_expr(condition, state)? != 1 {
                    return invalid("if condition must have width 1");
                }
                validate_actions(then_actions, state)?;
                validate_actions(else_actions, state)?;
            }
            Action::Write { state: id, value } => {
                let expected = state.get(id.as_str()).ok_or_else(|| {
                    Error::Validation(format!("write references unknown state {id:?}"))
                })?;
                let actual = validate_expr(value, state)?;
                if actual != *expected {
                    return invalid(format!(
                        "write to state {id:?} has {actual}-bit value; expected {expected}-bit value"
                    ));
                }
            }
            Action::Display { items } => {
                for item in items {
                    if let DisplayItem::Decimal { value, .. } = item {
                        validate_expr(value, state)?;
                    }
                }
            }
            Action::Finish { .. } => {}
        }
    }
    Ok(())
}

fn validate_expr(expr: &Expr, state: &BTreeMap<&str, u8>) -> Result<u8, Error> {
    match expr {
        Expr::State { id } => state.get(id.as_str()).copied().ok_or_else(|| {
            Error::Validation(format!("expression references unknown state {id:?}"))
        }),
        Expr::Const { width, value } => {
            validate_width(*width, "constant")?;
            if *value > value_mask(*width) {
                return invalid(format!("constant value does not fit its {width}-bit width"));
            }
            Ok(*width)
        }
        Expr::Binary { width, op, args } => {
            validate_width(*width, "binary expression")?;
            if args.len() != 2 {
                return invalid("binary expression must have exactly two arguments");
            }
            let left = validate_expr(&args[0], state)?;
            let right = validate_expr(&args[1], state)?;
            match op {
                BinaryOp::Add if left == right && *width == left => Ok(*width),
                BinaryOp::Equal | BinaryOp::UnsignedLessThan if left == right && *width == 1 => {
                    Ok(1)
                }
                BinaryOp::Add => invalid("add operands and result must have the same width"),
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
