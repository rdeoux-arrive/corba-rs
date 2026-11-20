use crate::corba::OctetSeq;
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

// IOR Profiles

// Standard Protocol Profile tag values
pub type ProfileId = u32;

pub type ProfileData = OctetSeq;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct TaggedProfile {
    pub tag: ProfileId,
    pub profile_data: ProfileData,
}

pub type TaggedProfileSeq = Vec<TaggedProfile>;

// The IOR

// an Interoperable Object Reference is a sequence of
// object-specific protocol profiles, plus a type ID.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct IOR {
    type_id: String,
    profiles: TaggedProfileSeq,
}

// IOR Components

pub type ObjectKey = OctetSeq;

// Service Contexts

pub type ServiceId = u32;
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ServiceContext {
    pub context_id: ServiceId,
    pub context_data: Vec<u8>,
}
pub type ServiceContextList = Vec<ServiceContext>;
