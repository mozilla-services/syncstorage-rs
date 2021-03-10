// This file is generated by rust-protobuf 2.22.0. Do not edit
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
//! Generated file from `google/type/phone_number.proto`

/// Generated files are compatible only with the same version
/// of protobuf runtime.
// const _PROTOBUF_VERSION_CHECK: () = ::protobuf::VERSION_2_22_0;

#[derive(PartialEq,Clone,Default)]
pub struct PhoneNumber {
    // message fields
    pub extension: ::std::string::String,
    // message oneof groups
    pub kind: ::std::option::Option<PhoneNumber_oneof_kind>,
    // special fields
    pub unknown_fields: ::protobuf::UnknownFields,
    pub cached_size: ::protobuf::CachedSize,
}

impl<'a> ::std::default::Default for &'a PhoneNumber {
    fn default() -> &'a PhoneNumber {
        <PhoneNumber as ::protobuf::Message>::default_instance()
    }
}

#[derive(Clone,PartialEq,Debug)]
pub enum PhoneNumber_oneof_kind {
    e164_number(::std::string::String),
    short_code(PhoneNumber_ShortCode),
}

impl PhoneNumber {
    pub fn new() -> PhoneNumber {
        ::std::default::Default::default()
    }

    // string e164_number = 1;


    pub fn get_e164_number(&self) -> &str {
        match self.kind {
            ::std::option::Option::Some(PhoneNumber_oneof_kind::e164_number(ref v)) => v,
            _ => "",
        }
    }
    pub fn clear_e164_number(&mut self) {
        self.kind = ::std::option::Option::None;
    }

    pub fn has_e164_number(&self) -> bool {
        match self.kind {
            ::std::option::Option::Some(PhoneNumber_oneof_kind::e164_number(..)) => true,
            _ => false,
        }
    }

    // Param is passed by value, moved
    pub fn set_e164_number(&mut self, v: ::std::string::String) {
        self.kind = ::std::option::Option::Some(PhoneNumber_oneof_kind::e164_number(v))
    }

    // Mutable pointer to the field.
    pub fn mut_e164_number(&mut self) -> &mut ::std::string::String {
        if let ::std::option::Option::Some(PhoneNumber_oneof_kind::e164_number(_)) = self.kind {
        } else {
            self.kind = ::std::option::Option::Some(PhoneNumber_oneof_kind::e164_number(::std::string::String::new()));
        }
        match self.kind {
            ::std::option::Option::Some(PhoneNumber_oneof_kind::e164_number(ref mut v)) => v,
            _ => panic!(),
        }
    }

    // Take field
    pub fn take_e164_number(&mut self) -> ::std::string::String {
        if self.has_e164_number() {
            match self.kind.take() {
                ::std::option::Option::Some(PhoneNumber_oneof_kind::e164_number(v)) => v,
                _ => panic!(),
            }
        } else {
            ::std::string::String::new()
        }
    }

    // .google.type.PhoneNumber.ShortCode short_code = 2;


    pub fn get_short_code(&self) -> &PhoneNumber_ShortCode {
        match self.kind {
            ::std::option::Option::Some(PhoneNumber_oneof_kind::short_code(ref v)) => v,
            _ => <PhoneNumber_ShortCode as ::protobuf::Message>::default_instance(),
        }
    }
    pub fn clear_short_code(&mut self) {
        self.kind = ::std::option::Option::None;
    }

    pub fn has_short_code(&self) -> bool {
        match self.kind {
            ::std::option::Option::Some(PhoneNumber_oneof_kind::short_code(..)) => true,
            _ => false,
        }
    }

    // Param is passed by value, moved
    pub fn set_short_code(&mut self, v: PhoneNumber_ShortCode) {
        self.kind = ::std::option::Option::Some(PhoneNumber_oneof_kind::short_code(v))
    }

    // Mutable pointer to the field.
    pub fn mut_short_code(&mut self) -> &mut PhoneNumber_ShortCode {
        if let ::std::option::Option::Some(PhoneNumber_oneof_kind::short_code(_)) = self.kind {
        } else {
            self.kind = ::std::option::Option::Some(PhoneNumber_oneof_kind::short_code(PhoneNumber_ShortCode::new()));
        }
        match self.kind {
            ::std::option::Option::Some(PhoneNumber_oneof_kind::short_code(ref mut v)) => v,
            _ => panic!(),
        }
    }

    // Take field
    pub fn take_short_code(&mut self) -> PhoneNumber_ShortCode {
        if self.has_short_code() {
            match self.kind.take() {
                ::std::option::Option::Some(PhoneNumber_oneof_kind::short_code(v)) => v,
                _ => panic!(),
            }
        } else {
            PhoneNumber_ShortCode::new()
        }
    }

    // string extension = 3;


    pub fn get_extension(&self) -> &str {
        &self.extension
    }
    pub fn clear_extension(&mut self) {
        self.extension.clear();
    }

    // Param is passed by value, moved
    pub fn set_extension(&mut self, v: ::std::string::String) {
        self.extension = v;
    }

    // Mutable pointer to the field.
    // If field is not initialized, it is initialized with default value first.
    pub fn mut_extension(&mut self) -> &mut ::std::string::String {
        &mut self.extension
    }

    // Take field
    pub fn take_extension(&mut self) -> ::std::string::String {
        ::std::mem::replace(&mut self.extension, ::std::string::String::new())
    }
}

impl ::protobuf::Message for PhoneNumber {
    fn is_initialized(&self) -> bool {
        if let Some(PhoneNumber_oneof_kind::short_code(ref v)) = self.kind {
            if !v.is_initialized() {
                return false;
            }
        }
        true
    }

    fn merge_from(&mut self, is: &mut ::protobuf::CodedInputStream<'_>) -> ::protobuf::ProtobufResult<()> {
        while !is.eof()? {
            let (field_number, wire_type) = is.read_tag_unpack()?;
            match field_number {
                1 => {
                    if wire_type != ::protobuf::wire_format::WireTypeLengthDelimited {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    self.kind = ::std::option::Option::Some(PhoneNumber_oneof_kind::e164_number(is.read_string()?));
                },
                2 => {
                    if wire_type != ::protobuf::wire_format::WireTypeLengthDelimited {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    self.kind = ::std::option::Option::Some(PhoneNumber_oneof_kind::short_code(is.read_message()?));
                },
                3 => {
                    ::protobuf::rt::read_singular_proto3_string_into(wire_type, is, &mut self.extension)?;
                },
                _ => {
                    ::protobuf::rt::read_unknown_or_skip_group(field_number, wire_type, is, self.mut_unknown_fields())?;
                },
            };
        }
        ::std::result::Result::Ok(())
    }

    // Compute sizes of nested messages
    #[allow(unused_variables)]
    fn compute_size(&self) -> u32 {
        let mut my_size = 0;
        if !self.extension.is_empty() {
            my_size += ::protobuf::rt::string_size(3, &self.extension);
        }
        if let ::std::option::Option::Some(ref v) = self.kind {
            match v {
                &PhoneNumber_oneof_kind::e164_number(ref v) => {
                    my_size += ::protobuf::rt::string_size(1, &v);
                },
                &PhoneNumber_oneof_kind::short_code(ref v) => {
                    let len = v.compute_size();
                    my_size += 1 + ::protobuf::rt::compute_raw_varint32_size(len) + len;
                },
            };
        }
        my_size += ::protobuf::rt::unknown_fields_size(self.get_unknown_fields());
        self.cached_size.set(my_size);
        my_size
    }

    fn write_to_with_cached_sizes(&self, os: &mut ::protobuf::CodedOutputStream<'_>) -> ::protobuf::ProtobufResult<()> {
        if !self.extension.is_empty() {
            os.write_string(3, &self.extension)?;
        }
        if let ::std::option::Option::Some(ref v) = self.kind {
            match v {
                &PhoneNumber_oneof_kind::e164_number(ref v) => {
                    os.write_string(1, v)?;
                },
                &PhoneNumber_oneof_kind::short_code(ref v) => {
                    os.write_tag(2, ::protobuf::wire_format::WireTypeLengthDelimited)?;
                    os.write_raw_varint32(v.get_cached_size())?;
                    v.write_to_with_cached_sizes(os)?;
                },
            };
        }
        os.write_unknown_fields(self.get_unknown_fields())?;
        ::std::result::Result::Ok(())
    }

    fn get_cached_size(&self) -> u32 {
        self.cached_size.get()
    }

    fn get_unknown_fields(&self) -> &::protobuf::UnknownFields {
        &self.unknown_fields
    }

    fn mut_unknown_fields(&mut self) -> &mut ::protobuf::UnknownFields {
        &mut self.unknown_fields
    }

    fn as_any(&self) -> &dyn (::std::any::Any) {
        self as &dyn (::std::any::Any)
    }
    fn as_any_mut(&mut self) -> &mut dyn (::std::any::Any) {
        self as &mut dyn (::std::any::Any)
    }
    fn into_any(self: ::std::boxed::Box<Self>) -> ::std::boxed::Box<dyn (::std::any::Any)> {
        self
    }

    fn descriptor(&self) -> &'static ::protobuf::reflect::MessageDescriptor {
        Self::descriptor_static()
    }

    fn new() -> PhoneNumber {
        PhoneNumber::new()
    }

    fn descriptor_static() -> &'static ::protobuf::reflect::MessageDescriptor {
        static descriptor: ::protobuf::rt::LazyV2<::protobuf::reflect::MessageDescriptor> = ::protobuf::rt::LazyV2::INIT;
        descriptor.get(|| {
            let mut fields = ::std::vec::Vec::new();
            fields.push(::protobuf::reflect::accessor::make_singular_string_accessor::<_>(
                "e164_number",
                PhoneNumber::has_e164_number,
                PhoneNumber::get_e164_number,
            ));
            fields.push(::protobuf::reflect::accessor::make_singular_message_accessor::<_, PhoneNumber_ShortCode>(
                "short_code",
                PhoneNumber::has_short_code,
                PhoneNumber::get_short_code,
            ));
            fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeString>(
                "extension",
                |m: &PhoneNumber| { &m.extension },
                |m: &mut PhoneNumber| { &mut m.extension },
            ));
            ::protobuf::reflect::MessageDescriptor::new_pb_name::<PhoneNumber>(
                "PhoneNumber",
                fields,
                file_descriptor_proto()
            )
        })
    }

    fn default_instance() -> &'static PhoneNumber {
        static instance: ::protobuf::rt::LazyV2<PhoneNumber> = ::protobuf::rt::LazyV2::INIT;
        instance.get(PhoneNumber::new)
    }
}

impl ::protobuf::Clear for PhoneNumber {
    fn clear(&mut self) {
        self.kind = ::std::option::Option::None;
        self.kind = ::std::option::Option::None;
        self.extension.clear();
        self.unknown_fields.clear();
    }
}

impl ::std::fmt::Debug for PhoneNumber {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        ::protobuf::text_format::fmt(self, f)
    }
}

impl ::protobuf::reflect::ProtobufValue for PhoneNumber {
    fn as_ref(&self) -> ::protobuf::reflect::ReflectValueRef {
        ::protobuf::reflect::ReflectValueRef::Message(self)
    }
}

#[derive(PartialEq,Clone,Default)]
pub struct PhoneNumber_ShortCode {
    // message fields
    pub region_code: ::std::string::String,
    pub number: ::std::string::String,
    // special fields
    pub unknown_fields: ::protobuf::UnknownFields,
    pub cached_size: ::protobuf::CachedSize,
}

impl<'a> ::std::default::Default for &'a PhoneNumber_ShortCode {
    fn default() -> &'a PhoneNumber_ShortCode {
        <PhoneNumber_ShortCode as ::protobuf::Message>::default_instance()
    }
}

impl PhoneNumber_ShortCode {
    pub fn new() -> PhoneNumber_ShortCode {
        ::std::default::Default::default()
    }

    // string region_code = 1;


    pub fn get_region_code(&self) -> &str {
        &self.region_code
    }
    pub fn clear_region_code(&mut self) {
        self.region_code.clear();
    }

    // Param is passed by value, moved
    pub fn set_region_code(&mut self, v: ::std::string::String) {
        self.region_code = v;
    }

    // Mutable pointer to the field.
    // If field is not initialized, it is initialized with default value first.
    pub fn mut_region_code(&mut self) -> &mut ::std::string::String {
        &mut self.region_code
    }

    // Take field
    pub fn take_region_code(&mut self) -> ::std::string::String {
        ::std::mem::replace(&mut self.region_code, ::std::string::String::new())
    }

    // string number = 2;


    pub fn get_number(&self) -> &str {
        &self.number
    }
    pub fn clear_number(&mut self) {
        self.number.clear();
    }

    // Param is passed by value, moved
    pub fn set_number(&mut self, v: ::std::string::String) {
        self.number = v;
    }

    // Mutable pointer to the field.
    // If field is not initialized, it is initialized with default value first.
    pub fn mut_number(&mut self) -> &mut ::std::string::String {
        &mut self.number
    }

    // Take field
    pub fn take_number(&mut self) -> ::std::string::String {
        ::std::mem::replace(&mut self.number, ::std::string::String::new())
    }
}

impl ::protobuf::Message for PhoneNumber_ShortCode {
    fn is_initialized(&self) -> bool {
        true
    }

    fn merge_from(&mut self, is: &mut ::protobuf::CodedInputStream<'_>) -> ::protobuf::ProtobufResult<()> {
        while !is.eof()? {
            let (field_number, wire_type) = is.read_tag_unpack()?;
            match field_number {
                1 => {
                    ::protobuf::rt::read_singular_proto3_string_into(wire_type, is, &mut self.region_code)?;
                },
                2 => {
                    ::protobuf::rt::read_singular_proto3_string_into(wire_type, is, &mut self.number)?;
                },
                _ => {
                    ::protobuf::rt::read_unknown_or_skip_group(field_number, wire_type, is, self.mut_unknown_fields())?;
                },
            };
        }
        ::std::result::Result::Ok(())
    }

    // Compute sizes of nested messages
    #[allow(unused_variables)]
    fn compute_size(&self) -> u32 {
        let mut my_size = 0;
        if !self.region_code.is_empty() {
            my_size += ::protobuf::rt::string_size(1, &self.region_code);
        }
        if !self.number.is_empty() {
            my_size += ::protobuf::rt::string_size(2, &self.number);
        }
        my_size += ::protobuf::rt::unknown_fields_size(self.get_unknown_fields());
        self.cached_size.set(my_size);
        my_size
    }

    fn write_to_with_cached_sizes(&self, os: &mut ::protobuf::CodedOutputStream<'_>) -> ::protobuf::ProtobufResult<()> {
        if !self.region_code.is_empty() {
            os.write_string(1, &self.region_code)?;
        }
        if !self.number.is_empty() {
            os.write_string(2, &self.number)?;
        }
        os.write_unknown_fields(self.get_unknown_fields())?;
        ::std::result::Result::Ok(())
    }

    fn get_cached_size(&self) -> u32 {
        self.cached_size.get()
    }

    fn get_unknown_fields(&self) -> &::protobuf::UnknownFields {
        &self.unknown_fields
    }

    fn mut_unknown_fields(&mut self) -> &mut ::protobuf::UnknownFields {
        &mut self.unknown_fields
    }

    fn as_any(&self) -> &dyn (::std::any::Any) {
        self as &dyn (::std::any::Any)
    }
    fn as_any_mut(&mut self) -> &mut dyn (::std::any::Any) {
        self as &mut dyn (::std::any::Any)
    }
    fn into_any(self: ::std::boxed::Box<Self>) -> ::std::boxed::Box<dyn (::std::any::Any)> {
        self
    }

    fn descriptor(&self) -> &'static ::protobuf::reflect::MessageDescriptor {
        Self::descriptor_static()
    }

    fn new() -> PhoneNumber_ShortCode {
        PhoneNumber_ShortCode::new()
    }

    fn descriptor_static() -> &'static ::protobuf::reflect::MessageDescriptor {
        static descriptor: ::protobuf::rt::LazyV2<::protobuf::reflect::MessageDescriptor> = ::protobuf::rt::LazyV2::INIT;
        descriptor.get(|| {
            let mut fields = ::std::vec::Vec::new();
            fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeString>(
                "region_code",
                |m: &PhoneNumber_ShortCode| { &m.region_code },
                |m: &mut PhoneNumber_ShortCode| { &mut m.region_code },
            ));
            fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeString>(
                "number",
                |m: &PhoneNumber_ShortCode| { &m.number },
                |m: &mut PhoneNumber_ShortCode| { &mut m.number },
            ));
            ::protobuf::reflect::MessageDescriptor::new_pb_name::<PhoneNumber_ShortCode>(
                "PhoneNumber.ShortCode",
                fields,
                file_descriptor_proto()
            )
        })
    }

    fn default_instance() -> &'static PhoneNumber_ShortCode {
        static instance: ::protobuf::rt::LazyV2<PhoneNumber_ShortCode> = ::protobuf::rt::LazyV2::INIT;
        instance.get(PhoneNumber_ShortCode::new)
    }
}

impl ::protobuf::Clear for PhoneNumber_ShortCode {
    fn clear(&mut self) {
        self.region_code.clear();
        self.number.clear();
        self.unknown_fields.clear();
    }
}

impl ::std::fmt::Debug for PhoneNumber_ShortCode {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        ::protobuf::text_format::fmt(self, f)
    }
}

impl ::protobuf::reflect::ProtobufValue for PhoneNumber_ShortCode {
    fn as_ref(&self) -> ::protobuf::reflect::ReflectValueRef {
        ::protobuf::reflect::ReflectValueRef::Message(self)
    }
}

static file_descriptor_proto_data: &'static [u8] = b"\
    \n\x1egoogle/type/phone_number.proto\x12\x0bgoogle.type\"\xe1\x01\n\x0bP\
    honeNumber\x12!\n\x0be164_number\x18\x01\x20\x01(\tH\0R\ne164Number\x12C\
    \n\nshort_code\x18\x02\x20\x01(\x0b2\".google.type.PhoneNumber.ShortCode\
    H\0R\tshortCode\x12\x1c\n\textension\x18\x03\x20\x01(\tR\textension\x1aD\
    \n\tShortCode\x12\x1f\n\x0bregion_code\x18\x01\x20\x01(\tR\nregionCode\
    \x12\x16\n\x06number\x18\x02\x20\x01(\tR\x06numberB\x06\n\x04kindBt\n\
    \x0fcom.google.typeB\x10PhoneNumberProtoP\x01ZDgoogle.golang.org/genprot\
    o/googleapis/type/phone_number;phone_number\xf8\x01\x01\xa2\x02\x03GTPJ\
    \x92#\n\x06\x12\x04\x0e\0p\x01\n\xbc\x04\n\x01\x0c\x12\x03\x0e\0\x122\
    \xb1\x04\x20Copyright\x202020\x20Google\x20LLC\n\n\x20Licensed\x20under\
    \x20the\x20Apache\x20License,\x20Version\x202.0\x20(the\x20\"License\");\
    \n\x20you\x20may\x20not\x20use\x20this\x20file\x20except\x20in\x20compli\
    ance\x20with\x20the\x20License.\n\x20You\x20may\x20obtain\x20a\x20copy\
    \x20of\x20the\x20License\x20at\n\n\x20\x20\x20\x20\x20http://www.apache.\
    org/licenses/LICENSE-2.0\n\n\x20Unless\x20required\x20by\x20applicable\
    \x20law\x20or\x20agreed\x20to\x20in\x20writing,\x20software\n\x20distrib\
    uted\x20under\x20the\x20License\x20is\x20distributed\x20on\x20an\x20\"AS\
    \x20IS\"\x20BASIS,\n\x20WITHOUT\x20WARRANTIES\x20OR\x20CONDITIONS\x20OF\
    \x20ANY\x20KIND,\x20either\x20express\x20or\x20implied.\n\x20See\x20the\
    \x20License\x20for\x20the\x20specific\x20language\x20governing\x20permis\
    sions\x20and\n\x20limitations\x20under\x20the\x20License.\n\n\x08\n\x01\
    \x02\x12\x03\x10\0\x14\n\x08\n\x01\x08\x12\x03\x12\0\x1f\n\t\n\x02\x08\
    \x1f\x12\x03\x12\0\x1f\n\x08\n\x01\x08\x12\x03\x13\0[\n\t\n\x02\x08\x0b\
    \x12\x03\x13\0[\n\x08\n\x01\x08\x12\x03\x14\0\"\n\t\n\x02\x08\n\x12\x03\
    \x14\0\"\n\x08\n\x01\x08\x12\x03\x15\01\n\t\n\x02\x08\x08\x12\x03\x15\01\
    \n\x08\n\x01\x08\x12\x03\x16\0(\n\t\n\x02\x08\x01\x12\x03\x16\0(\n\x08\n\
    \x01\x08\x12\x03\x17\0!\n\t\n\x02\x08$\x12\x03\x17\0!\n\xf8\x07\n\x02\
    \x04\0\x12\x043\0p\x01\x1a\xeb\x07\x20An\x20object\x20representing\x20a\
    \x20phone\x20number,\x20suitable\x20as\x20an\x20API\x20wire\x20format.\n\
    \n\x20This\x20representation:\n\n\x20\x20-\x20should\x20not\x20be\x20use\
    d\x20for\x20locale-specific\x20formatting\x20of\x20a\x20phone\x20number,\
    \x20such\n\x20\x20\x20\x20as\x20\"+1\x20(650)\x20253-0000\x20ext.\x20123\
    \"\n\n\x20\x20-\x20is\x20not\x20designed\x20for\x20efficient\x20storage\
    \n\x20\x20-\x20may\x20not\x20be\x20suitable\x20for\x20dialing\x20-\x20sp\
    ecialized\x20libraries\x20(see\x20references)\n\x20\x20\x20\x20should\
    \x20be\x20used\x20to\x20parse\x20the\x20number\x20for\x20that\x20purpose\
    \n\n\x20To\x20do\x20something\x20meaningful\x20with\x20this\x20number,\
    \x20such\x20as\x20format\x20it\x20for\x20various\n\x20use-cases,\x20conv\
    ert\x20it\x20to\x20an\x20`i18n.phonenumbers.PhoneNumber`\x20object\x20fi\
    rst.\n\n\x20For\x20instance,\x20in\x20Java\x20this\x20would\x20be:\n\n\
    \x20\x20\x20\x20com.google.type.PhoneNumber\x20wireProto\x20=\n\x20\x20\
    \x20\x20\x20\x20\x20\x20com.google.type.PhoneNumber.newBuilder().build()\
    ;\n\x20\x20\x20\x20com.google.i18n.phonenumbers.Phonenumber.PhoneNumber\
    \x20phoneNumber\x20=\n\x20\x20\x20\x20\x20\x20\x20\x20PhoneNumberUtil.ge\
    tInstance().parse(wireProto.getE164Number(),\x20\"ZZ\");\n\x20\x20\x20\
    \x20if\x20(!wireProto.getExtension().isEmpty())\x20{\n\x20\x20\x20\x20\
    \x20\x20phoneNumber.setExtension(wireProto.getExtension());\n\x20\x20\
    \x20\x20}\n\n\x20\x20Reference(s):\n\x20\x20\x20-\x20https://github.com/\
    google/libphonenumber\n\n\n\n\x03\x04\0\x01\x12\x033\x08\x13\n\xb5\x04\n\
    \x04\x04\0\x03\0\x12\x04=\x02H\x03\x1a\xa6\x04\x20An\x20object\x20repres\
    enting\x20a\x20short\x20code,\x20which\x20is\x20a\x20phone\x20number\x20\
    that\x20is\n\x20typically\x20much\x20shorter\x20than\x20regular\x20phone\
    \x20numbers\x20and\x20can\x20be\x20used\x20to\n\x20address\x20messages\
    \x20in\x20MMS\x20and\x20SMS\x20systems,\x20as\x20well\x20as\x20for\x20ab\
    breviated\x20dialing\n\x20(e.g.\x20\"Text\x20611\x20to\x20see\x20how\x20\
    many\x20minutes\x20you\x20have\x20remaining\x20on\x20your\x20plan.\").\n\
    \n\x20Short\x20codes\x20are\x20restricted\x20to\x20a\x20region\x20and\
    \x20are\x20not\x20internationally\n\x20dialable,\x20which\x20means\x20th\
    e\x20same\x20short\x20code\x20can\x20exist\x20in\x20different\x20regions\
    ,\n\x20with\x20different\x20usage\x20and\x20pricing,\x20even\x20if\x20th\
    ose\x20regions\x20share\x20the\x20same\n\x20country\x20calling\x20code\
    \x20(e.g.\x20US\x20and\x20CA).\n\n\x0c\n\x05\x04\0\x03\0\x01\x12\x03=\n\
    \x13\n\xd5\x01\n\x06\x04\0\x03\0\x02\0\x12\x03C\x04\x1b\x1a\xc5\x01\x20R\
    equired.\x20The\x20BCP-47\x20region\x20code\x20of\x20the\x20location\x20\
    where\x20calls\x20to\x20this\n\x20short\x20code\x20can\x20be\x20made,\
    \x20such\x20as\x20\"US\"\x20and\x20\"BB\".\n\n\x20Reference(s):\n\x20\
    \x20-\x20http://www.unicode.org/reports/tr35/#unicode_region_subtag\n\n\
    \x0f\n\x07\x04\0\x03\0\x02\0\x04\x12\x04C\x04=\x15\n\x0e\n\x07\x04\0\x03\
    \0\x02\0\x05\x12\x03C\x04\n\n\x0e\n\x07\x04\0\x03\0\x02\0\x01\x12\x03C\
    \x0b\x16\n\x0e\n\x07\x04\0\x03\0\x02\0\x03\x12\x03C\x19\x1a\nt\n\x06\x04\
    \0\x03\0\x02\x01\x12\x03G\x04\x16\x1ae\x20Required.\x20The\x20short\x20c\
    ode\x20digits,\x20without\x20a\x20leading\x20plus\x20('+')\x20or\x20coun\
    try\n\x20calling\x20code,\x20e.g.\x20\"611\".\n\n\x0f\n\x07\x04\0\x03\0\
    \x02\x01\x04\x12\x04G\x04C\x1b\n\x0e\n\x07\x04\0\x03\0\x02\x01\x05\x12\
    \x03G\x04\n\n\x0e\n\x07\x04\0\x03\0\x02\x01\x01\x12\x03G\x0b\x11\n\x0e\n\
    \x07\x04\0\x03\0\x02\x01\x03\x12\x03G\x14\x15\n\xe2\x01\n\x04\x04\0\x08\
    \0\x12\x04M\x02d\x03\x1a\xd3\x01\x20Required.\x20\x20Either\x20a\x20regu\
    lar\x20number,\x20or\x20a\x20short\x20code.\x20\x20New\x20fields\x20may\
    \x20be\n\x20added\x20to\x20the\x20oneof\x20below\x20in\x20the\x20future,\
    \x20so\x20clients\x20should\x20ignore\x20phone\n\x20numbers\x20for\x20wh\
    ich\x20none\x20of\x20the\x20fields\x20they\x20coded\x20against\x20are\
    \x20set.\n\n\x0c\n\x05\x04\0\x08\0\x01\x12\x03M\x08\x0c\n\xb0\x05\n\x04\
    \x04\0\x02\0\x12\x03]\x04\x1b\x1a\xa2\x05\x20The\x20phone\x20number,\x20\
    represented\x20as\x20a\x20leading\x20plus\x20sign\x20('+'),\x20followed\
    \x20by\x20a\n\x20phone\x20number\x20that\x20uses\x20a\x20relaxed\x20ITU\
    \x20E.164\x20format\x20consisting\x20of\x20the\n\x20country\x20calling\
    \x20code\x20(1\x20to\x203\x20digits)\x20and\x20the\x20subscriber\x20numb\
    er,\x20with\x20no\n\x20additional\x20spaces\x20or\x20formatting,\x20e.g.\
    :\n\x20\x20-\x20correct:\x20\"+15552220123\"\n\x20\x20-\x20incorrect:\
    \x20\"+1\x20(555)\x20222-01234\x20x123\".\n\n\x20The\x20ITU\x20E.164\x20\
    format\x20limits\x20the\x20latter\x20to\x2012\x20digits,\x20but\x20in\
    \x20practice\x20not\n\x20all\x20countries\x20respect\x20that,\x20so\x20w\
    e\x20relax\x20that\x20restriction\x20here.\n\x20National-only\x20numbers\
    \x20are\x20not\x20allowed.\n\n\x20References:\n\x20\x20-\x20https://www.\
    itu.int/rec/T-REC-E.164-201011-I\n\x20\x20-\x20https://en.wikipedia.org/\
    wiki/E.164.\n\x20\x20-\x20https://en.wikipedia.org/wiki/List_of_country_\
    calling_codes\n\n\x0c\n\x05\x04\0\x02\0\x05\x12\x03]\x04\n\n\x0c\n\x05\
    \x04\0\x02\0\x01\x12\x03]\x0b\x16\n\x0c\n\x05\x04\0\x02\0\x03\x12\x03]\
    \x19\x1a\nY\n\x04\x04\0\x02\x01\x12\x03c\x04\x1d\x1aL\x20A\x20short\x20c\
    ode.\n\n\x20Reference(s):\n\x20\x20-\x20https://en.wikipedia.org/wiki/Sh\
    ort_code\n\n\x0c\n\x05\x04\0\x02\x01\x06\x12\x03c\x04\r\n\x0c\n\x05\x04\
    \0\x02\x01\x01\x12\x03c\x0e\x18\n\x0c\n\x05\x04\0\x02\x01\x03\x12\x03c\
    \x1b\x1c\n\x95\x04\n\x04\x04\0\x02\x02\x12\x03o\x02\x17\x1a\x87\x04\x20T\
    he\x20phone\x20number's\x20extension.\x20The\x20extension\x20is\x20not\
    \x20standardized\x20in\x20ITU\n\x20recommendations,\x20except\x20for\x20\
    being\x20defined\x20as\x20a\x20series\x20of\x20numbers\x20with\x20a\n\
    \x20maximum\x20length\x20of\x2040\x20digits.\x20Other\x20than\x20digits,\
    \x20some\x20other\x20dialing\n\x20characters\x20such\x20as\x20','\x20(in\
    dicating\x20a\x20wait)\x20or\x20'#'\x20may\x20be\x20stored\x20here.\n\n\
    \x20Note\x20that\x20no\x20regions\x20currently\x20use\x20extensions\x20w\
    ith\x20short\x20codes,\x20so\x20this\n\x20field\x20is\x20normally\x20onl\
    y\x20set\x20in\x20conjunction\x20with\x20an\x20E.164\x20number.\x20It\
    \x20is\x20held\n\x20separately\x20from\x20the\x20E.164\x20number\x20to\
    \x20allow\x20for\x20short\x20code\x20extensions\x20in\x20the\n\x20future\
    .\n\n\r\n\x05\x04\0\x02\x02\x04\x12\x04o\x02d\x03\n\x0c\n\x05\x04\0\x02\
    \x02\x05\x12\x03o\x02\x08\n\x0c\n\x05\x04\0\x02\x02\x01\x12\x03o\t\x12\n\
    \x0c\n\x05\x04\0\x02\x02\x03\x12\x03o\x15\x16b\x06proto3\
";

static file_descriptor_proto_lazy: ::protobuf::rt::LazyV2<::protobuf::descriptor::FileDescriptorProto> = ::protobuf::rt::LazyV2::INIT;

fn parse_descriptor_proto() -> ::protobuf::descriptor::FileDescriptorProto {
    ::protobuf::Message::parse_from_bytes(file_descriptor_proto_data).unwrap()
}

pub fn file_descriptor_proto() -> &'static ::protobuf::descriptor::FileDescriptorProto {
    file_descriptor_proto_lazy.get(|| {
        parse_descriptor_proto()
    })
}
