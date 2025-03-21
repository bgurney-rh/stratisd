// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::{
    error::Error,
    fmt::{self, Display},
    str::FromStr,
};

use clap::{builder::PossibleValuesParser, Arg, ArgAction, Command};

#[cfg(feature = "systemd_compat")]
use clap::builder::Str;
use log::LevelFilter;
use strum::VariantNames;

use devicemapper::Bytes;

use stratisd::engine::{
    IntegritySpec, IntegrityTagSpec, ValidatedIntegritySpec, DEFAULT_INTEGRITY_JOURNAL_SIZE,
    DEFAULT_INTEGRITY_TAG_SPEC,
};

use crate::utils::predict_usage;

#[cfg(feature = "systemd_compat")]
use crate::utils::generators::{stratis_clevis_setup_generator, stratis_setup_generator};

#[derive(Debug)]
pub struct ExecutableError(pub String);

impl Display for ExecutableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for ExecutableError {}

pub trait UtilCommand<'a> {
    fn name(&self) -> &'a str;
    fn run(&self, command_line_args: Vec<String>) -> Result<(), Box<dyn Error>>;
}

struct StratisPredictUsage;

impl StratisPredictUsage {
    fn cmd() -> Command {
        Command::new("stratis-predict-usage")
            .about("Predicts space usage for Stratis.")
            .arg(
                Arg::new("log-level")
                .value_parser(["off", "error", "warn", "info", "debug", "trace"])
                .default_value("off")
                .long("log-level")
                .help("Sets level for generation of log messages"),
            )
            .subcommand_required(true)
            .subcommands(vec![
                Command::new("pool")
                    .about("Predicts the space usage when creating a Stratis pool.")
                    .arg(Arg::new("encrypted")
                        .long("encrypted")
                        .action(ArgAction::SetTrue)
                        .help("Whether the pool will be encrypted.")
                        .long_help(
"Since space for crypt metadata is allocated regardless of whether or not the
pool is encrypted, setting this option has no effect on the prediction."),
                    )
                    .arg(
                        Arg::new("no-overprovision")
                        .long("no-overprovision")
                        .action(ArgAction::SetTrue)
                        .help("Indicates that the pool does not allow overprovisioning"),
                    )
                    .arg(
                        Arg::new("device-size")
                            .long("device-size")
                            .num_args(1)
                            .action(ArgAction::Append)
                            .required(true)
                            .value_parser(clap::value_parser!(u128))
                            .help("Size of device to be included in the pool. May be specified multiple times. Units are bytes.")
                            .next_line_help(true)
                    )
                    .arg(
                        Arg::new("filesystem-size")
                        .long("filesystem-size")
                        .num_args(1)
                        .action(ArgAction::Append)
                        .value_parser(clap::value_parser!(u128))
                        .help("Size of filesystem to be made for this pool. May be specified multiple times, one for each filesystem. Units are bytes. Must be at least 512 MiB and less than 4 PiB.")
                        .next_line_help(true)
                    )
                    .arg(
                        Arg::new("integrity_tag_spec")
                        .long("integrity-tag-spec")
                        .num_args(1)
                        .value_parser(PossibleValuesParser::new(IntegrityTagSpec::VARIANTS))
                        .default_value(DEFAULT_INTEGRITY_TAG_SPEC.as_ref())
                        .help("Integrity tag specification defining the size of the tag to store a checksum or other value for each block on a device.")
                        .next_line_help(true)
                    )
                    .arg(
                        Arg::new("integrity_journal_size")
                        .long("integrity-journal-size")
                        .num_args(1)
                        .value_parser(clap::value_parser!(u64))
                        .default_value(Box::leak((*DEFAULT_INTEGRITY_JOURNAL_SIZE).to_string().into_boxed_str()) as &'static str)
                        .help("Size of the integrity journal. Default is 128 MiB. Units are bytes.")
                        .next_line_help(true)
                    )
                    .arg(
                        Arg::new("no_integrity_superblock")
                        .action(ArgAction::SetTrue)
                        .long("no-integrity-superblock")
                        .help("Do not allocate space for integrity superblock")
                        .next_line_help(true)
                    ),
                Command::new("filesystem")
                    .about("Predicts the space usage when creating a Stratis filesystem.")
                    .arg(
                        Arg::new("filesystem-size")
                        .long("filesystem-size")
                        .num_args(1)
                        .action(ArgAction::Append)
                        .value_parser(clap::value_parser!(u128))
                        .required(true)
                        .help("Size of filesystem to be made for this pool. May be specified multiple times, one for each filesystem. Units are bytes. Must be at least 512 MiB and less than 4 PiB.")
                        .next_line_help(true)
                    )
                    .arg(
                        Arg::new("no-overprovision")
                        .long("no-overprovision")
                        .action(ArgAction::SetTrue)
                        .help("Indicates that the pool does not allow overprovisioning"),
                    )]
            )
    }
}

impl<'a> UtilCommand<'a> for StratisPredictUsage {
    fn name(&self) -> &'a str {
        "stratis-predict-usage"
    }

    fn run(&self, command_line_args: Vec<String>) -> Result<(), Box<dyn Error>> {
        let matches = StratisPredictUsage::cmd().get_matches_from(command_line_args);
        match matches.subcommand() {
            Some(("pool", sub_m)) => predict_usage::predict_pool_usage(
                !sub_m.get_flag("no-overprovision"),
                sub_m
                    .get_many::<u128>("device-size")
                    .map(|szs| szs.map(|n| Bytes(*n)).collect::<Vec<_>>())
                    .expect("required argument"),
                sub_m
                    .get_many::<u128>("filesystem-size")
                    .map(|szs| szs.map(|n| Bytes(*n)).collect::<Vec<_>>()),
                ValidatedIntegritySpec::try_from(IntegritySpec {
                    journal_size: Some(
                        sub_m
                            .get_one::<u64>("integrity_journal_size")
                            .map(|n| Bytes::from(*n))
                            .expect("default specified by parser"),
                    ),
                    tag_spec: Some(
                        sub_m
                            .get_one::<String>("integrity_tag_spec")
                            .map(|sz| {
                                IntegrityTagSpec::try_from(sz.as_str())
                                    .expect("parser ensures valid value")
                            })
                            .expect("default specified by parser"),
                    ),
                    allocate_superblock: Some(!sub_m.get_flag("no_integrity_superblock")),
                })?,
                LevelFilter::from_str(
                    matches
                        .get_one::<String>("log-level")
                        .expect("default value set"),
                )
                .expect("only valid entries allowed"),
            ),
            Some(("filesystem", sub_m)) => predict_usage::predict_filesystem_usage(
                !sub_m.get_flag("no-overprovision"),
                sub_m
                    .get_many::<u128>("filesystem-size")
                    .map(|szs| szs.map(|n| Bytes(*n)).collect::<Vec<_>>())
                    .expect("required argument"),
                LevelFilter::from_str(
                    matches
                        .get_one::<String>("log-level")
                        .expect("default value set"),
                )
                .expect("only valid entries allowed"),
            ),
            _ => unreachable!("Impossible subcommand name"),
        }
    }
}

#[cfg(feature = "systemd_compat")]
fn stratis_setup_generator_cmd(generator: impl Into<Str>) -> Command {
    Command::new(generator)
        .arg(
            Arg::new("normal_priority_dir")
                .required(true)
                .help("Directory in which to write a unit file for normal priority"),
        )
        .arg(
            Arg::new("early_priority_dir")
                .required(true)
                .help("Directory in which to write a unit file for early priority"),
        )
        .arg(
            Arg::new("late_priority_dir")
                .required(true)
                .help("Directory in which to write a unit file for late priority"),
        )
}

struct StratisSetupGenerator;

impl<'a> UtilCommand<'a> for StratisSetupGenerator {
    fn name(&self) -> &'a str {
        "stratis-setup-generator"
    }

    #[cfg(feature = "systemd_compat")]
    fn run(&self, command_line_args: Vec<String>) -> Result<(), Box<dyn Error>> {
        let matches = stratis_setup_generator_cmd("stratis-setup-generator")
            .get_matches_from(command_line_args);

        stratis_setup_generator::generator(
            matches
                .get_one::<String>("early_priority_dir")
                .expect("required")
                .to_owned(),
        )
    }

    #[cfg(not(feature = "systemd_compat"))]
    fn run(&self, _command_line_args: Vec<String>) -> Result<(), Box<dyn Error>> {
        Err(Box::new(ExecutableError(
            "systemd compatibility disabled for this build".into(),
        )))
    }
}

struct StratisClevisSetupGenerator;

impl<'a> UtilCommand<'a> for StratisClevisSetupGenerator {
    fn name(&self) -> &'a str {
        "stratis-clevis-setup-generator"
    }

    #[cfg(feature = "systemd_compat")]
    fn run(&self, command_line_args: Vec<String>) -> Result<(), Box<dyn Error>> {
        let matches = stratis_setup_generator_cmd("stratis-clevis-setup-generator")
            .get_matches_from(command_line_args);

        stratis_clevis_setup_generator::generator(
            matches
                .get_one::<String>("early_priority_dir")
                .expect("required")
                .to_owned(),
        )
    }

    #[cfg(not(feature = "systemd_compat"))]
    fn run(&self, _command_line_args: Vec<String>) -> Result<(), Box<dyn Error>> {
        Err(Box::new(ExecutableError(
            "systemd compatibility disabled for this build".into(),
        )))
    }
}

pub fn cmds<'a>() -> Vec<Box<dyn UtilCommand<'a>>> {
    vec![
        Box::new(StratisPredictUsage),
        Box::new(StratisSetupGenerator),
        Box::new(StratisClevisSetupGenerator),
    ]
}

#[cfg(test)]
mod tests {

    use super::StratisPredictUsage;

    #[cfg(feature = "systemd_compat")]
    use super::stratis_setup_generator_cmd;

    #[test]
    fn test_predictusage_parse_args() {
        StratisPredictUsage::cmd().debug_assert();
    }

    #[test]
    #[cfg(feature = "systemd_compat")]
    fn test_generator_parse_args() {
        stratis_setup_generator_cmd("stratis-generator").debug_assert();
    }
}
