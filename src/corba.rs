use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionStatus {
    CompletedYes,
    CompletedNo,
    CompletedMaybe,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExceptionType {
    NoException,
    UserException,
    SystemException,
}

macro_rules! exception {
    ($n:ident) => {
        pub struct $n {
            pub minor: u32,
            pub completed: CompletionStatus,
        }
    };
}

// the unknown exception
exception!(Unknown);

// an invalid parameter was
// passed
exception!(BadParam);

// dynamic memory allocation
// failure
exception!(NoMemory);

// violated implementation
// limit
exception!(ImpLimit);

// communication failure
exception!(CommFailure);

// invalid object reference
exception!(InvObjref);

// no permission for
// attempted op.
exception!(NoPermission);

// ORB internal error
exception!(Internal);

// error marshaling
// param/result
exception!(Marshal);

// ORB initialization failure
exception!(Initialize);

// operation implementation
// unavailable
exception!(NoImplement);

// bad typecode
exception!(BadTypecode);

// invalid operation
exception!(BadOperation);

// insufficient resources
// for req.
exception!(NoResources);

// response to req. not yet
// available
exception!(NoResponse);

// persistent storage failure
exception!(PersistStore);

// routine invocations
// out of order
exception!(BadInvOrder);

// transient failure - reissue
// request
exception!(Transient);

// cannot free memory
exception!(FreeMem);

// invalid identifier syntax
exception!(InvIdent);

// invalid flag was specified
exception!(InvFlag);

// error accessing interface
// repository
exception!(IntfRepos);

// error processing context
// object
exception!(BadContext);

// failure detected by object
// adapter
exception!(ObjAdapter);

// data conversion error
exception!(DataConversion);

// non-existent object,
// delete reference
exception!(ObjectNotExist);

// transaction required
exception!(TransactionRequired);

// transaction rolled back
exception!(TransactionRolledback);

// invalid transaction
exception!(InvalidTransaction);

// invalid policy
exception!(InvPolicy);

// incompatible code set
exception!(CodesetIncompatible);

// rebind needed
exception!(Rebind);

// operation timed out
exception!(Timeout);

// no transaction
exception!(TransactionUnavailable);

// invalid transaction mode
exception!(TransactionMode);

// bad quality of service
exception!(BadQos);

pub type OctetSeq = Vec<u8>;
