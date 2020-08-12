// This file is generated by rust-protobuf 2.17.0. Do not edit
// @generated

// https://github.com/rust-lang/rust-clippy/issues/702
#![allow(unknown_lints)]
#![allow(clippy::all)]

#![allow(unused_attributes)]
#![rustfmt::skip]

#![allow(box_pointers)]
#![allow(dead_code)]
#![allow(missing_docs)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(trivial_casts)]
#![allow(unused_imports)]
#![allow(unused_results)]
//! Generated file from `google/rpc/code.proto`

/// Generated files are compatible only with the same version
/// of protobuf runtime.
// const _PROTOBUF_VERSION_CHECK: () = ::protobuf::VERSION_2_17_0;

#[derive(Clone,PartialEq,Eq,Debug,Hash)]
pub enum Code {
    OK = 0,
    CANCELLED = 1,
    UNKNOWN = 2,
    INVALID_ARGUMENT = 3,
    DEADLINE_EXCEEDED = 4,
    NOT_FOUND = 5,
    ALREADY_EXISTS = 6,
    PERMISSION_DENIED = 7,
    UNAUTHENTICATED = 16,
    RESOURCE_EXHAUSTED = 8,
    FAILED_PRECONDITION = 9,
    ABORTED = 10,
    OUT_OF_RANGE = 11,
    UNIMPLEMENTED = 12,
    INTERNAL = 13,
    UNAVAILABLE = 14,
    DATA_LOSS = 15,
}

impl ::protobuf::ProtobufEnum for Code {
    fn value(&self) -> i32 {
        *self as i32
    }

    fn from_i32(value: i32) -> ::std::option::Option<Code> {
        match value {
            0 => ::std::option::Option::Some(Code::OK),
            1 => ::std::option::Option::Some(Code::CANCELLED),
            2 => ::std::option::Option::Some(Code::UNKNOWN),
            3 => ::std::option::Option::Some(Code::INVALID_ARGUMENT),
            4 => ::std::option::Option::Some(Code::DEADLINE_EXCEEDED),
            5 => ::std::option::Option::Some(Code::NOT_FOUND),
            6 => ::std::option::Option::Some(Code::ALREADY_EXISTS),
            7 => ::std::option::Option::Some(Code::PERMISSION_DENIED),
            16 => ::std::option::Option::Some(Code::UNAUTHENTICATED),
            8 => ::std::option::Option::Some(Code::RESOURCE_EXHAUSTED),
            9 => ::std::option::Option::Some(Code::FAILED_PRECONDITION),
            10 => ::std::option::Option::Some(Code::ABORTED),
            11 => ::std::option::Option::Some(Code::OUT_OF_RANGE),
            12 => ::std::option::Option::Some(Code::UNIMPLEMENTED),
            13 => ::std::option::Option::Some(Code::INTERNAL),
            14 => ::std::option::Option::Some(Code::UNAVAILABLE),
            15 => ::std::option::Option::Some(Code::DATA_LOSS),
            _ => ::std::option::Option::None
        }
    }

    fn values() -> &'static [Self] {
        static values: &'static [Code] = &[
            Code::OK,
            Code::CANCELLED,
            Code::UNKNOWN,
            Code::INVALID_ARGUMENT,
            Code::DEADLINE_EXCEEDED,
            Code::NOT_FOUND,
            Code::ALREADY_EXISTS,
            Code::PERMISSION_DENIED,
            Code::UNAUTHENTICATED,
            Code::RESOURCE_EXHAUSTED,
            Code::FAILED_PRECONDITION,
            Code::ABORTED,
            Code::OUT_OF_RANGE,
            Code::UNIMPLEMENTED,
            Code::INTERNAL,
            Code::UNAVAILABLE,
            Code::DATA_LOSS,
        ];
        values
    }

    fn enum_descriptor_static() -> &'static ::protobuf::reflect::EnumDescriptor {
        static descriptor: ::protobuf::rt::LazyV2<::protobuf::reflect::EnumDescriptor> = ::protobuf::rt::LazyV2::INIT;
        descriptor.get(|| {
            ::protobuf::reflect::EnumDescriptor::new_pb_name::<Code>("Code", file_descriptor_proto())
        })
    }
}

impl ::std::marker::Copy for Code {
}

impl ::std::default::Default for Code {
    fn default() -> Self {
        Code::OK
    }
}

impl ::protobuf::reflect::ProtobufValue for Code {
    fn as_ref(&self) -> ::protobuf::reflect::ReflectValueRef {
        ::protobuf::reflect::ReflectValueRef::Enum(::protobuf::ProtobufEnum::descriptor(self))
    }
}

static file_descriptor_proto_data: &'static [u8] = b"\
    \n\x15google/rpc/code.proto\x12\ngoogle.rpc*\xb7\x02\n\x04Code\x12\x06\n\
    \x02OK\x10\0\x12\r\n\tCANCELLED\x10\x01\x12\x0b\n\x07UNKNOWN\x10\x02\x12\
    \x14\n\x10INVALID_ARGUMENT\x10\x03\x12\x15\n\x11DEADLINE_EXCEEDED\x10\
    \x04\x12\r\n\tNOT_FOUND\x10\x05\x12\x12\n\x0eALREADY_EXISTS\x10\x06\x12\
    \x15\n\x11PERMISSION_DENIED\x10\x07\x12\x13\n\x0fUNAUTHENTICATED\x10\x10\
    \x12\x16\n\x12RESOURCE_EXHAUSTED\x10\x08\x12\x17\n\x13FAILED_PRECONDITIO\
    N\x10\t\x12\x0b\n\x07ABORTED\x10\n\x12\x10\n\x0cOUT_OF_RANGE\x10\x0b\x12\
    \x11\n\rUNIMPLEMENTED\x10\x0c\x12\x0c\n\x08INTERNAL\x10\r\x12\x0f\n\x0bU\
    NAVAILABLE\x10\x0e\x12\r\n\tDATA_LOSS\x10\x0fBX\n\x0ecom.google.rpcB\tCo\
    deProtoP\x01Z3google.golang.org/genproto/googleapis/rpc/code;code\xa2\
    \x02\x03RPCJ\xb45\n\x07\x12\x05\x0e\0\xb9\x01\x01\n\xbd\x04\n\x01\x0c\
    \x12\x03\x0e\0\x122\xb2\x04\x20Copyright\x202017\x20Google\x20Inc.\n\n\
    \x20Licensed\x20under\x20the\x20Apache\x20License,\x20Version\x202.0\x20\
    (the\x20\"License\");\n\x20you\x20may\x20not\x20use\x20this\x20file\x20e\
    xcept\x20in\x20compliance\x20with\x20the\x20License.\n\x20You\x20may\x20\
    obtain\x20a\x20copy\x20of\x20the\x20License\x20at\n\n\x20\x20\x20\x20\
    \x20http://www.apache.org/licenses/LICENSE-2.0\n\n\x20Unless\x20required\
    \x20by\x20applicable\x20law\x20or\x20agreed\x20to\x20in\x20writing,\x20s\
    oftware\n\x20distributed\x20under\x20the\x20License\x20is\x20distributed\
    \x20on\x20an\x20\"AS\x20IS\"\x20BASIS,\n\x20WITHOUT\x20WARRANTIES\x20OR\
    \x20CONDITIONS\x20OF\x20ANY\x20KIND,\x20either\x20express\x20or\x20impli\
    ed.\n\x20See\x20the\x20License\x20for\x20the\x20specific\x20language\x20\
    governing\x20permissions\x20and\n\x20limitations\x20under\x20the\x20Lice\
    nse.\n\n\x08\n\x01\x02\x12\x03\x10\0\x13\n\x08\n\x01\x08\x12\x03\x12\0J\
    \n\t\n\x02\x08\x0b\x12\x03\x12\0J\n\x08\n\x01\x08\x12\x03\x13\0\"\n\t\n\
    \x02\x08\n\x12\x03\x13\0\"\n\x08\n\x01\x08\x12\x03\x14\0*\n\t\n\x02\x08\
    \x08\x12\x03\x14\0*\n\x08\n\x01\x08\x12\x03\x15\0'\n\t\n\x02\x08\x01\x12\
    \x03\x15\0'\n\x08\n\x01\x08\x12\x03\x16\0!\n\t\n\x02\x08$\x12\x03\x16\0!\
    \n\xce\x02\n\x02\x05\0\x12\x05\x20\0\xb9\x01\x01\x1a\xc0\x02\x20The\x20c\
    anonical\x20error\x20codes\x20for\x20Google\x20APIs.\n\n\n\x20Sometimes\
    \x20multiple\x20error\x20codes\x20may\x20apply.\x20\x20Services\x20shoul\
    d\x20return\n\x20the\x20most\x20specific\x20error\x20code\x20that\x20app\
    lies.\x20\x20For\x20example,\x20prefer\n\x20`OUT_OF_RANGE`\x20over\x20`F\
    AILED_PRECONDITION`\x20if\x20both\x20codes\x20apply.\n\x20Similarly\x20p\
    refer\x20`NOT_FOUND`\x20or\x20`ALREADY_EXISTS`\x20over\x20`FAILED_PRECON\
    DITION`.\n\n\n\n\x03\x05\0\x01\x12\x03\x20\x05\t\nG\n\x04\x05\0\x02\0\
    \x12\x03$\x02\t\x1a:\x20Not\x20an\x20error;\x20returned\x20on\x20success\
    \n\n\x20HTTP\x20Mapping:\x20200\x20OK\n\n\x0c\n\x05\x05\0\x02\0\x01\x12\
    \x03$\x02\x04\n\x0c\n\x05\x05\0\x02\0\x02\x12\x03$\x07\x08\nn\n\x04\x05\
    \0\x02\x01\x12\x03)\x02\x10\x1aa\x20The\x20operation\x20was\x20cancelled\
    ,\x20typically\x20by\x20the\x20caller.\n\n\x20HTTP\x20Mapping:\x20499\
    \x20Client\x20Closed\x20Request\n\n\x0c\n\x05\x05\0\x02\x01\x01\x12\x03)\
    \x02\x0b\n\x0c\n\x05\x05\0\x02\x01\x02\x12\x03)\x0e\x0f\n\xda\x02\n\x04\
    \x05\0\x02\x02\x12\x032\x02\x0e\x1a\xcc\x02\x20Unknown\x20error.\x20\x20\
    For\x20example,\x20this\x20error\x20may\x20be\x20returned\x20when\n\x20a\
    \x20`Status`\x20value\x20received\x20from\x20another\x20address\x20space\
    \x20belongs\x20to\n\x20an\x20error\x20space\x20that\x20is\x20not\x20know\
    n\x20in\x20this\x20address\x20space.\x20\x20Also\n\x20errors\x20raised\
    \x20by\x20APIs\x20that\x20do\x20not\x20return\x20enough\x20error\x20info\
    rmation\n\x20may\x20be\x20converted\x20to\x20this\x20error.\n\n\x20HTTP\
    \x20Mapping:\x20500\x20Internal\x20Server\x20Error\n\n\x0c\n\x05\x05\0\
    \x02\x02\x01\x12\x032\x02\t\n\x0c\n\x05\x05\0\x02\x02\x02\x12\x032\x0c\r\
    \n\x92\x02\n\x04\x05\0\x02\x03\x12\x03:\x02\x17\x1a\x84\x02\x20The\x20cl\
    ient\x20specified\x20an\x20invalid\x20argument.\x20\x20Note\x20that\x20t\
    his\x20differs\n\x20from\x20`FAILED_PRECONDITION`.\x20\x20`INVALID_ARGUM\
    ENT`\x20indicates\x20arguments\n\x20that\x20are\x20problematic\x20regard\
    less\x20of\x20the\x20state\x20of\x20the\x20system\n\x20(e.g.,\x20a\x20ma\
    lformed\x20file\x20name).\n\n\x20HTTP\x20Mapping:\x20400\x20Bad\x20Reque\
    st\n\n\x0c\n\x05\x05\0\x02\x03\x01\x12\x03:\x02\x12\n\x0c\n\x05\x05\0\
    \x02\x03\x02\x12\x03:\x15\x16\n\xe4\x02\n\x04\x05\0\x02\x04\x12\x03C\x02\
    \x18\x1a\xd6\x02\x20The\x20deadline\x20expired\x20before\x20the\x20opera\
    tion\x20could\x20complete.\x20For\x20operations\n\x20that\x20change\x20t\
    he\x20state\x20of\x20the\x20system,\x20this\x20error\x20may\x20be\x20ret\
    urned\n\x20even\x20if\x20the\x20operation\x20has\x20completed\x20success\
    fully.\x20\x20For\x20example,\x20a\n\x20successful\x20response\x20from\
    \x20a\x20server\x20could\x20have\x20been\x20delayed\x20long\n\x20enough\
    \x20for\x20the\x20deadline\x20to\x20expire.\n\n\x20HTTP\x20Mapping:\x205\
    04\x20Gateway\x20Timeout\n\n\x0c\n\x05\x05\0\x02\x04\x01\x12\x03C\x02\
    \x13\n\x0c\n\x05\x05\0\x02\x04\x02\x12\x03C\x16\x17\n\x9a\x03\n\x04\x05\
    \0\x02\x05\x12\x03N\x02\x10\x1a\x8c\x03\x20Some\x20requested\x20entity\
    \x20(e.g.,\x20file\x20or\x20directory)\x20was\x20not\x20found.\n\n\x20No\
    te\x20to\x20server\x20developers:\x20if\x20a\x20request\x20is\x20denied\
    \x20for\x20an\x20entire\x20class\n\x20of\x20users,\x20such\x20as\x20grad\
    ual\x20feature\x20rollout\x20or\x20undocumented\x20whitelist,\n\x20`NOT_\
    FOUND`\x20may\x20be\x20used.\x20If\x20a\x20request\x20is\x20denied\x20fo\
    r\x20some\x20users\x20within\n\x20a\x20class\x20of\x20users,\x20such\x20\
    as\x20user-based\x20access\x20control,\x20`PERMISSION_DENIED`\n\x20must\
    \x20be\x20used.\n\n\x20HTTP\x20Mapping:\x20404\x20Not\x20Found\n\n\x0c\n\
    \x05\x05\0\x02\x05\x01\x12\x03N\x02\x0b\n\x0c\n\x05\x05\0\x02\x05\x02\
    \x12\x03N\x0e\x0f\n\x83\x01\n\x04\x05\0\x02\x06\x12\x03T\x02\x15\x1av\
    \x20The\x20entity\x20that\x20a\x20client\x20attempted\x20to\x20create\
    \x20(e.g.,\x20file\x20or\x20directory)\n\x20already\x20exists.\n\n\x20HT\
    TP\x20Mapping:\x20409\x20Conflict\n\n\x0c\n\x05\x05\0\x02\x06\x01\x12\
    \x03T\x02\x10\n\x0c\n\x05\x05\0\x02\x06\x02\x12\x03T\x13\x14\n\xf9\x03\n\
    \x04\x05\0\x02\x07\x12\x03`\x02\x18\x1a\xeb\x03\x20The\x20caller\x20does\
    \x20not\x20have\x20permission\x20to\x20execute\x20the\x20specified\n\x20\
    operation.\x20`PERMISSION_DENIED`\x20must\x20not\x20be\x20used\x20for\
    \x20rejections\n\x20caused\x20by\x20exhausting\x20some\x20resource\x20(u\
    se\x20`RESOURCE_EXHAUSTED`\n\x20instead\x20for\x20those\x20errors).\x20`\
    PERMISSION_DENIED`\x20must\x20not\x20be\n\x20used\x20if\x20the\x20caller\
    \x20can\x20not\x20be\x20identified\x20(use\x20`UNAUTHENTICATED`\n\x20ins\
    tead\x20for\x20those\x20errors).\x20This\x20error\x20code\x20does\x20not\
    \x20imply\x20the\n\x20request\x20is\x20valid\x20or\x20the\x20requested\
    \x20entity\x20exists\x20or\x20satisfies\n\x20other\x20pre-conditions.\n\
    \n\x20HTTP\x20Mapping:\x20403\x20Forbidden\n\n\x0c\n\x05\x05\0\x02\x07\
    \x01\x12\x03`\x02\x13\n\x0c\n\x05\x05\0\x02\x07\x02\x12\x03`\x16\x17\n~\
    \n\x04\x05\0\x02\x08\x12\x03f\x02\x17\x1aq\x20The\x20request\x20does\x20\
    not\x20have\x20valid\x20authentication\x20credentials\x20for\x20the\n\
    \x20operation.\n\n\x20HTTP\x20Mapping:\x20401\x20Unauthorized\n\n\x0c\n\
    \x05\x05\0\x02\x08\x01\x12\x03f\x02\x11\n\x0c\n\x05\x05\0\x02\x08\x02\
    \x12\x03f\x14\x16\n\xa5\x01\n\x04\x05\0\x02\t\x12\x03l\x02\x19\x1a\x97\
    \x01\x20Some\x20resource\x20has\x20been\x20exhausted,\x20perhaps\x20a\
    \x20per-user\x20quota,\x20or\n\x20perhaps\x20the\x20entire\x20file\x20sy\
    stem\x20is\x20out\x20of\x20space.\n\n\x20HTTP\x20Mapping:\x20429\x20Too\
    \x20Many\x20Requests\n\n\x0c\n\x05\x05\0\x02\t\x01\x12\x03l\x02\x14\n\
    \x0c\n\x05\x05\0\x02\t\x02\x12\x03l\x17\x18\n\xd9\x07\n\x04\x05\0\x02\n\
    \x12\x04\x80\x01\x02\x1a\x1a\xca\x07\x20The\x20operation\x20was\x20rejec\
    ted\x20because\x20the\x20system\x20is\x20not\x20in\x20a\x20state\n\x20re\
    quired\x20for\x20the\x20operation's\x20execution.\x20\x20For\x20example,\
    \x20the\x20directory\n\x20to\x20be\x20deleted\x20is\x20non-empty,\x20an\
    \x20rmdir\x20operation\x20is\x20applied\x20to\n\x20a\x20non-directory,\
    \x20etc.\n\n\x20Service\x20implementors\x20can\x20use\x20the\x20followin\
    g\x20guidelines\x20to\x20decide\n\x20between\x20`FAILED_PRECONDITION`,\
    \x20`ABORTED`,\x20and\x20`UNAVAILABLE`:\n\x20\x20(a)\x20Use\x20`UNAVAILA\
    BLE`\x20if\x20the\x20client\x20can\x20retry\x20just\x20the\x20failing\
    \x20call.\n\x20\x20(b)\x20Use\x20`ABORTED`\x20if\x20the\x20client\x20sho\
    uld\x20retry\x20at\x20a\x20higher\x20level\n\x20\x20\x20\x20\x20\x20(e.g\
    .,\x20when\x20a\x20client-specified\x20test-and-set\x20fails,\x20indicat\
    ing\x20the\n\x20\x20\x20\x20\x20\x20client\x20should\x20restart\x20a\x20\
    read-modify-write\x20sequence).\n\x20\x20(c)\x20Use\x20`FAILED_PRECONDIT\
    ION`\x20if\x20the\x20client\x20should\x20not\x20retry\x20until\n\x20\x20\
    \x20\x20\x20\x20the\x20system\x20state\x20has\x20been\x20explicitly\x20f\
    ixed.\x20\x20E.g.,\x20if\x20an\x20\"rmdir\"\n\x20\x20\x20\x20\x20\x20fai\
    ls\x20because\x20the\x20directory\x20is\x20non-empty,\x20`FAILED_PRECOND\
    ITION`\n\x20\x20\x20\x20\x20\x20should\x20be\x20returned\x20since\x20the\
    \x20client\x20should\x20not\x20retry\x20unless\n\x20\x20\x20\x20\x20\x20\
    the\x20files\x20are\x20deleted\x20from\x20the\x20directory.\n\n\x20HTTP\
    \x20Mapping:\x20400\x20Bad\x20Request\n\n\r\n\x05\x05\0\x02\n\x01\x12\
    \x04\x80\x01\x02\x15\n\r\n\x05\x05\0\x02\n\x02\x12\x04\x80\x01\x18\x19\n\
    \x8c\x02\n\x04\x05\0\x02\x0b\x12\x04\x89\x01\x02\x0f\x1a\xfd\x01\x20The\
    \x20operation\x20was\x20aborted,\x20typically\x20due\x20to\x20a\x20concu\
    rrency\x20issue\x20such\x20as\n\x20a\x20sequencer\x20check\x20failure\
    \x20or\x20transaction\x20abort.\n\n\x20See\x20the\x20guidelines\x20above\
    \x20for\x20deciding\x20between\x20`FAILED_PRECONDITION`,\n\x20`ABORTED`,\
    \x20and\x20`UNAVAILABLE`.\n\n\x20HTTP\x20Mapping:\x20409\x20Conflict\n\n\
    \r\n\x05\x05\0\x02\x0b\x01\x12\x04\x89\x01\x02\t\n\r\n\x05\x05\0\x02\x0b\
    \x02\x12\x04\x89\x01\x0c\x0e\n\x85\x06\n\x04\x05\0\x02\x0c\x12\x04\x9c\
    \x01\x02\x14\x1a\xf6\x05\x20The\x20operation\x20was\x20attempted\x20past\
    \x20the\x20valid\x20range.\x20\x20E.g.,\x20seeking\x20or\n\x20reading\
    \x20past\x20end-of-file.\n\n\x20Unlike\x20`INVALID_ARGUMENT`,\x20this\
    \x20error\x20indicates\x20a\x20problem\x20that\x20may\n\x20be\x20fixed\
    \x20if\x20the\x20system\x20state\x20changes.\x20For\x20example,\x20a\x20\
    32-bit\x20file\n\x20system\x20will\x20generate\x20`INVALID_ARGUMENT`\x20\
    if\x20asked\x20to\x20read\x20at\x20an\n\x20offset\x20that\x20is\x20not\
    \x20in\x20the\x20range\x20[0,2^32-1],\x20but\x20it\x20will\x20generate\n\
    \x20`OUT_OF_RANGE`\x20if\x20asked\x20to\x20read\x20from\x20an\x20offset\
    \x20past\x20the\x20current\n\x20file\x20size.\n\n\x20There\x20is\x20a\
    \x20fair\x20bit\x20of\x20overlap\x20between\x20`FAILED_PRECONDITION`\x20\
    and\n\x20`OUT_OF_RANGE`.\x20\x20We\x20recommend\x20using\x20`OUT_OF_RANG\
    E`\x20(the\x20more\x20specific\n\x20error)\x20when\x20it\x20applies\x20s\
    o\x20that\x20callers\x20who\x20are\x20iterating\x20through\n\x20a\x20spa\
    ce\x20can\x20easily\x20look\x20for\x20an\x20`OUT_OF_RANGE`\x20error\x20t\
    o\x20detect\x20when\n\x20they\x20are\x20done.\n\n\x20HTTP\x20Mapping:\
    \x20400\x20Bad\x20Request\n\n\r\n\x05\x05\0\x02\x0c\x01\x12\x04\x9c\x01\
    \x02\x0e\n\r\n\x05\x05\0\x02\x0c\x02\x12\x04\x9c\x01\x11\x13\n\x82\x01\n\
    \x04\x05\0\x02\r\x12\x04\xa2\x01\x02\x15\x1at\x20The\x20operation\x20is\
    \x20not\x20implemented\x20or\x20is\x20not\x20supported/enabled\x20in\x20\
    this\n\x20service.\n\n\x20HTTP\x20Mapping:\x20501\x20Not\x20Implemented\
    \n\n\r\n\x05\x05\0\x02\r\x01\x12\x04\xa2\x01\x02\x0f\n\r\n\x05\x05\0\x02\
    \r\x02\x12\x04\xa2\x01\x12\x14\n\xd3\x01\n\x04\x05\0\x02\x0e\x12\x04\xa9\
    \x01\x02\x10\x1a\xc4\x01\x20Internal\x20errors.\x20\x20This\x20means\x20\
    that\x20some\x20invariants\x20expected\x20by\x20the\n\x20underlying\x20s\
    ystem\x20have\x20been\x20broken.\x20\x20This\x20error\x20code\x20is\x20r\
    eserved\n\x20for\x20serious\x20errors.\n\n\x20HTTP\x20Mapping:\x20500\
    \x20Internal\x20Server\x20Error\n\n\r\n\x05\x05\0\x02\x0e\x01\x12\x04\
    \xa9\x01\x02\n\n\r\n\x05\x05\0\x02\x0e\x02\x12\x04\xa9\x01\r\x0f\n\xa5\
    \x02\n\x04\x05\0\x02\x0f\x12\x04\xb3\x01\x02\x13\x1a\x96\x02\x20The\x20s\
    ervice\x20is\x20currently\x20unavailable.\x20\x20This\x20is\x20most\x20l\
    ikely\x20a\n\x20transient\x20condition,\x20which\x20can\x20be\x20correct\
    ed\x20by\x20retrying\x20with\n\x20a\x20backoff.\n\n\x20See\x20the\x20gui\
    delines\x20above\x20for\x20deciding\x20between\x20`FAILED_PRECONDITION`,\
    \n\x20`ABORTED`,\x20and\x20`UNAVAILABLE`.\n\n\x20HTTP\x20Mapping:\x20503\
    \x20Service\x20Unavailable\n\n\r\n\x05\x05\0\x02\x0f\x01\x12\x04\xb3\x01\
    \x02\r\n\r\n\x05\x05\0\x02\x0f\x02\x12\x04\xb3\x01\x10\x12\n`\n\x04\x05\
    \0\x02\x10\x12\x04\xb8\x01\x02\x11\x1aR\x20Unrecoverable\x20data\x20loss\
    \x20or\x20corruption.\n\n\x20HTTP\x20Mapping:\x20500\x20Internal\x20Serv\
    er\x20Error\n\n\r\n\x05\x05\0\x02\x10\x01\x12\x04\xb8\x01\x02\x0b\n\r\n\
    \x05\x05\0\x02\x10\x02\x12\x04\xb8\x01\x0e\x10b\x06proto3\
";

static file_descriptor_proto_lazy: ::protobuf::rt::LazyV2<::protobuf::descriptor::FileDescriptorProto> = ::protobuf::rt::LazyV2::INIT;

fn parse_descriptor_proto() -> ::protobuf::descriptor::FileDescriptorProto {
    ::protobuf::parse_from_bytes(file_descriptor_proto_data).unwrap()
}

pub fn file_descriptor_proto() -> &'static ::protobuf::descriptor::FileDescriptorProto {
    file_descriptor_proto_lazy.get(|| {
        parse_descriptor_proto()
    })
}
