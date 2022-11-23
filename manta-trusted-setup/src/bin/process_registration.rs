// Copyright 2019-2022 Manta Network.
// This file is part of manta-rs.
//
// manta-rs is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// manta-rs is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with manta-rs.  If not, see <http://www.gnu.org/licenses/>.

//! Trusted Setup Ceremony Server
//! Run as
//! cargo run --release --all-features --bin process_registration -- path_to_registration_form_v1 path_to_registration_form_v2 path_to_server_registry_file

// for local server test
// cargo run --release --all-features --bin process_registration -- /Users/thomascnorton/Desktop/ts_signups_1.csv /Users/thomascnorton/Desktop/ts_signups_2.csv /Users/thomascnorton/Documents/Manta/manta-rs/manta-trusted-setup/data/registry.csv
use clap::Parser;
use manta_trusted_setup::groth16::ceremony::{
    config::ppot::{extract_registry, Config, Record},
    CeremonyError,
};
use manta_util::serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// These are the headers in the .csv automatically generated by the
/// registration form.
const EXPECTED_HEADERS: [&str; 19]= [
    "First up, what\'s your first name?", 
    "What is your email address? ", 
    "Okay {{field:c393dfe5f7faa4de}}, what your signature?", 
    "What\'s your public key, {{field:c393dfe5f7faa4de}}?", 
    "Finally, what\'s your Twitter Handle?  ", 
    "Alright {{field:c393dfe5f7faa4de}}, why is privacy important to you?", 
    "We want to reward participation with a POAP designed to commemorate this historical Web 3 achievement. If you would like to receive one please share your wallet address", 
    "score", 
    "Finally, what\'s your Twitter Handle?  ", 
    "What\'s your public key, {{field:c393dfe5f7faa4de}}?", 
    "What\'s your Discord ID, {{field:c393dfe5f7faa4de}}?", 
    "What is your motivation for participating in the Trusted Setup, {{field:c393dfe5f7faa4de}}?",
    "Where are you from?",
    "email", "twitter", "verifying_key", "signature",
    "Submitted At", 
    "Token"
];

const EXPECTED_HEADERS_V2: [&str; 17] = [
    "First up, what should we call you?", 
    "What is your email address? ", 
    "Okay {{field:c393dfe5f7faa4de}}, what your signature?", 
    "What\'s your public key, {{field:c393dfe5f7faa4de}}?", 
    "What\'s your Discord ID, {{field:c393dfe5f7faa4de}}?", 
    "Finally, what\'s your Twitter Handle?  ", 
    "Alright {{field:c393dfe5f7faa4de}}, why is privacy important to you?", 
    "What is your motivation for participating in the Trusted Setup?", 
    "Where are you from?", 
    "We want to reward participation with an NFT designed to commemorate this historical Web 3 achievement. If you would like to receive one please share your KMA address.", 
    "Lastly, how did you find out about the Trusted Setup?", 
    "twitter", 
    "email", 
    "verifying_key", 
    "signature", 
    "Submitted At", 
    "Token",
];

/// Short versions of the above headers to allow
/// rows of the .csv to be deserialized to [`RegistrationInfoV1`].
const SHORT_HEADERS: [&str; 19] = [
    "name",
    "email",
    "signature",
    "verifying_key_null",
    "twitter_null",
    "why_privacy",
    "wallet",
    "score",
    "twitter",
    "verifying_key",
    "discord",
    "motivation",
    "where_from",
    "email_hidden",
    "twitter_hidden",
    "verifying_key_hidden",
    "signature_hidden",
    "submission_time",
    "submission_token",
];

const SHORT_HEADERS_V2: [&str; 17] = [
    "name",
    "comms_email",
    "unused_signature",
    "unused_verifying_key",
    "discord",
    "unused_twitter",
    "why_privacy",
    "motivation",
    "where_from",
    "wallet",
    "heard_where",
    "twitter",
    "email",
    "verifying_key",
    "signature",
    "submission_time",
    "submission_token",
];

/// Server CLI
#[derive(Debug, Parser)]
pub struct Arguments {
    raw_registry_v1_path: String,

    raw_registry_v2_path: String,

    #[clap(default_value = "manta-trusted-setup/data/registry.csv")]
    registry_path: String,
}

impl Arguments {
    /// Runs a server.
    #[inline]
    pub fn run(self) -> Result<(), CeremonyError<Config>> {
        // Parse from Registration Form v1
        let priority_list = HashMap::new();
        let (successful, malformed) = extract_registry::<RegistrationInfoV1>(
            self.raw_registry_v1_path.into(),
            self.registry_path.clone().into(),
            EXPECTED_HEADERS.into(),
            SHORT_HEADERS.into(),
            priority_list.clone(),
        )
        .expect("Registry processing failed");
        println!(
            "Processed registration form v1.
            Processed a total of {} registry entries. \
            {successful} were processed successfully. \
            {malformed} were malformed entries.",
            successful + malformed,
        );
        // Parse from Registration Form v2
        let (successful, malformed) = extract_registry::<RegistrationInfoV2>(
            self.raw_registry_v2_path.into(),
            self.registry_path.into(),
            EXPECTED_HEADERS_V2.into(),
            SHORT_HEADERS_V2.into(),
            priority_list,
        )
        .expect("Registry processing failed");
        println!(
            "Processed registration form v2.
            Processed a total of {} registry entries. \
            {successful} were processed successfully. \
            {malformed} were malformed entries.",
            successful + malformed,
        );
        Ok(())
    }
}

fn main() {
    Arguments::parse().run().expect("Server error occurred");
}

/// Registration info collected by our registration form.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(
    bound(deserialize = "", serialize = ""),
    crate = "manta_util::serde",
    deny_unknown_fields
)]
pub struct RegistrationInfoV1 {
    /// First name (may be empty)
    pub name: String,

    /// Email Account
    pub email: String,

    /// Signature
    pub signature: String,

    /// Verifying Key
    pub verifying_key: String,

    /// Twitter Account
    pub twitter: String,

    /// Why is privacy important (may be empty)
    pub why_privacy: String,

    /// Wallet address (may be empty)
    pub wallet: String,

    /// Score
    pub score: String,

    /// Named field that's always empty
    pub twitter_null: String,

    /// Named field that's always empty
    pub verifying_key_null: String,

    /// Discord ID (may be empty)
    pub discord: String,

    /// Motivation to participate (may be empty)
    pub motivation: String,

    /// Where are you from? (may be empty)
    pub where_from: String,

    /// Twitter Hidden Variable
    twitter_hidden: String,

    /// Email Hidden Variable
    email_hidden: String,

    /// Verifying Key Hidden Variable
    verifying_key_hidden: String,

    /// Signature Hidden Variable
    signature_hidden: String,

    /// Submission time
    pub submission_time: String,

    /// Submission token
    pub submission_token: String,
}

impl From<RegistrationInfoV1> for Record {
    fn from(value: RegistrationInfoV1) -> Self {
        Self::new(
            value.twitter,
            value.email,
            "true".to_string(),
            value.verifying_key,
            value.signature,
        )
    }
}

/// Registration info collected by our registration form.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(
    bound(deserialize = "", serialize = ""),
    crate = "manta_util::serde",
    deny_unknown_fields
)]
pub struct RegistrationInfoV2 {
    // Participant Name (optional)
    pub name: String,

    /// Email for communications, not signature verification
    pub comms_email: String,

    /// Unused field that previously held signature
    pub unused_signature: String,

    /// Unused field that previously held verifying key
    pub unused_verifying_key: String,

    /// Discord ID (may be empty)
    pub discord: String,

    /// Unused field that previously held twitter
    pub unused_twitter: String,

    /// Why is privacy important (may be empty)
    pub why_privacy: String,

    /// Motivation to participate (may be empty)
    pub motivation: String,

    /// Where are you from? (may be empty)
    pub where_from: String,

    /// Wallet address (may be empty)
    pub wallet: String,

    /// Where the participant heard about TS ceremony
    pub heard_where: String,

    /// Twitter Account
    pub twitter: String,

    /// Email Address
    pub email: String,

    /// Verifying Key
    pub verifying_key: String,

    /// Signature
    pub signature: String,

    /// Submission time
    pub submission_time: String,

    /// Submission token
    pub submission_token: String,
}

impl From<RegistrationInfoV2> for Record {
    fn from(value: RegistrationInfoV2) -> Self {
        Self::new(
            value.twitter,
            value.email,
            "true".to_string(),
            value.verifying_key,
            value.signature,
        )
    }
}
