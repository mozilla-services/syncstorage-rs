// This file is generated by rust-protobuf 2.25.2. Do not edit
// @generated

// https://github.com/rust-lang/rust-clippy/issues/702
#![allow(unknown_lints)]
#![allow(clippy::all)]

#![allow(unused_attributes)]
#![cfg_attr(rustfmt, rustfmt::skip)]

#![allow(box_pointers)]
#![allow(dead_code)]
#![allow(missing_docs)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(trivial_casts)]
#![allow(unused_imports)]
#![allow(unused_results)]
//! Generated file from `google/type/quaternion.proto`

/// Generated files are compatible only with the same version
/// of protobuf runtime.
// const _PROTOBUF_VERSION_CHECK: () = ::protobuf::VERSION_2_25_2;

#[derive(PartialEq,Clone,Default)]
pub struct Quaternion {
    // message fields
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
    // special fields
    pub unknown_fields: ::protobuf::UnknownFields,
    pub cached_size: ::protobuf::CachedSize,
}

impl<'a> ::std::default::Default for &'a Quaternion {
    fn default() -> &'a Quaternion {
        <Quaternion as ::protobuf::Message>::default_instance()
    }
}

impl Quaternion {
    pub fn new() -> Quaternion {
        ::std::default::Default::default()
    }

    // double x = 1;


    pub fn get_x(&self) -> f64 {
        self.x
    }
    pub fn clear_x(&mut self) {
        self.x = 0.;
    }

    // Param is passed by value, moved
    pub fn set_x(&mut self, v: f64) {
        self.x = v;
    }

    // double y = 2;


    pub fn get_y(&self) -> f64 {
        self.y
    }
    pub fn clear_y(&mut self) {
        self.y = 0.;
    }

    // Param is passed by value, moved
    pub fn set_y(&mut self, v: f64) {
        self.y = v;
    }

    // double z = 3;


    pub fn get_z(&self) -> f64 {
        self.z
    }
    pub fn clear_z(&mut self) {
        self.z = 0.;
    }

    // Param is passed by value, moved
    pub fn set_z(&mut self, v: f64) {
        self.z = v;
    }

    // double w = 4;


    pub fn get_w(&self) -> f64 {
        self.w
    }
    pub fn clear_w(&mut self) {
        self.w = 0.;
    }

    // Param is passed by value, moved
    pub fn set_w(&mut self, v: f64) {
        self.w = v;
    }
}

impl ::protobuf::Message for Quaternion {
    fn is_initialized(&self) -> bool {
        true
    }

    fn merge_from(&mut self, is: &mut ::protobuf::CodedInputStream<'_>) -> ::protobuf::ProtobufResult<()> {
        while !is.eof()? {
            let (field_number, wire_type) = is.read_tag_unpack()?;
            match field_number {
                1 => {
                    if wire_type != ::protobuf::wire_format::WireTypeFixed64 {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    let tmp = is.read_double()?;
                    self.x = tmp;
                },
                2 => {
                    if wire_type != ::protobuf::wire_format::WireTypeFixed64 {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    let tmp = is.read_double()?;
                    self.y = tmp;
                },
                3 => {
                    if wire_type != ::protobuf::wire_format::WireTypeFixed64 {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    let tmp = is.read_double()?;
                    self.z = tmp;
                },
                4 => {
                    if wire_type != ::protobuf::wire_format::WireTypeFixed64 {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    let tmp = is.read_double()?;
                    self.w = tmp;
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
        if self.x != 0. {
            my_size += 9;
        }
        if self.y != 0. {
            my_size += 9;
        }
        if self.z != 0. {
            my_size += 9;
        }
        if self.w != 0. {
            my_size += 9;
        }
        my_size += ::protobuf::rt::unknown_fields_size(self.get_unknown_fields());
        self.cached_size.set(my_size);
        my_size
    }

    fn write_to_with_cached_sizes(&self, os: &mut ::protobuf::CodedOutputStream<'_>) -> ::protobuf::ProtobufResult<()> {
        if self.x != 0. {
            os.write_double(1, self.x)?;
        }
        if self.y != 0. {
            os.write_double(2, self.y)?;
        }
        if self.z != 0. {
            os.write_double(3, self.z)?;
        }
        if self.w != 0. {
            os.write_double(4, self.w)?;
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

    fn new() -> Quaternion {
        Quaternion::new()
    }

    fn descriptor_static() -> &'static ::protobuf::reflect::MessageDescriptor {
        static descriptor: ::protobuf::rt::LazyV2<::protobuf::reflect::MessageDescriptor> = ::protobuf::rt::LazyV2::INIT;
        descriptor.get(|| {
            let mut fields = ::std::vec::Vec::new();
            fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeDouble>(
                "x",
                |m: &Quaternion| { &m.x },
                |m: &mut Quaternion| { &mut m.x },
            ));
            fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeDouble>(
                "y",
                |m: &Quaternion| { &m.y },
                |m: &mut Quaternion| { &mut m.y },
            ));
            fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeDouble>(
                "z",
                |m: &Quaternion| { &m.z },
                |m: &mut Quaternion| { &mut m.z },
            ));
            fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeDouble>(
                "w",
                |m: &Quaternion| { &m.w },
                |m: &mut Quaternion| { &mut m.w },
            ));
            ::protobuf::reflect::MessageDescriptor::new_pb_name::<Quaternion>(
                "Quaternion",
                fields,
                file_descriptor_proto()
            )
        })
    }

    fn default_instance() -> &'static Quaternion {
        static instance: ::protobuf::rt::LazyV2<Quaternion> = ::protobuf::rt::LazyV2::INIT;
        instance.get(Quaternion::new)
    }
}

impl ::protobuf::Clear for Quaternion {
    fn clear(&mut self) {
        self.x = 0.;
        self.y = 0.;
        self.z = 0.;
        self.w = 0.;
        self.unknown_fields.clear();
    }
}

impl ::std::fmt::Debug for Quaternion {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        ::protobuf::text_format::fmt(self, f)
    }
}

impl ::protobuf::reflect::ProtobufValue for Quaternion {
    fn as_ref(&self) -> ::protobuf::reflect::ReflectValueRef {
        ::protobuf::reflect::ReflectValueRef::Message(self)
    }
}

static file_descriptor_proto_data: &'static [u8] = b"\
    \n\x1cgoogle/type/quaternion.proto\x12\x0bgoogle.type\"D\n\nQuaternion\
    \x12\x0c\n\x01x\x18\x01\x20\x01(\x01R\x01x\x12\x0c\n\x01y\x18\x02\x20\
    \x01(\x01R\x01y\x12\x0c\n\x01z\x18\x03\x20\x01(\x01R\x01z\x12\x0c\n\x01w\
    \x18\x04\x20\x01(\x01R\x01wBo\n\x0fcom.google.typeB\x0fQuaternionProtoP\
    \x01Z@google.golang.org/genproto/googleapis/type/quaternion;quaternion\
    \xf8\x01\x01\xa2\x02\x03GTPJ\xf4\x1c\n\x06\x12\x04\x0f\0^\x01\n\xbe\x04\
    \n\x01\x0c\x12\x03\x0f\0\x122\xb3\x04\x20Copyright\x202019\x20Google\x20\
    LLC.\n\n\x20Licensed\x20under\x20the\x20Apache\x20License,\x20Version\
    \x202.0\x20(the\x20\"License\");\n\x20you\x20may\x20not\x20use\x20this\
    \x20file\x20except\x20in\x20compliance\x20with\x20the\x20License.\n\x20Y\
    ou\x20may\x20obtain\x20a\x20copy\x20of\x20the\x20License\x20at\n\n\x20\
    \x20\x20\x20\x20http://www.apache.org/licenses/LICENSE-2.0\n\n\x20Unless\
    \x20required\x20by\x20applicable\x20law\x20or\x20agreed\x20to\x20in\x20w\
    riting,\x20software\n\x20distributed\x20under\x20the\x20License\x20is\
    \x20distributed\x20on\x20an\x20\"AS\x20IS\"\x20BASIS,\n\x20WITHOUT\x20WA\
    RRANTIES\x20OR\x20CONDITIONS\x20OF\x20ANY\x20KIND,\x20either\x20express\
    \x20or\x20implied.\n\x20See\x20the\x20License\x20for\x20the\x20specific\
    \x20language\x20governing\x20permissions\x20and\n\x20limitations\x20unde\
    r\x20the\x20License.\n\n\n\x08\n\x01\x02\x12\x03\x11\x08\x13\n\x08\n\x01\
    \x08\x12\x03\x13\0\x1f\n\t\n\x02\x08\x1f\x12\x03\x13\0\x1f\n\x08\n\x01\
    \x08\x12\x03\x14\0W\n\t\n\x02\x08\x0b\x12\x03\x14\0W\n\x08\n\x01\x08\x12\
    \x03\x15\0\"\n\t\n\x02\x08\n\x12\x03\x15\0\"\n\x08\n\x01\x08\x12\x03\x16\
    \00\n\t\n\x02\x08\x08\x12\x03\x16\00\n\x08\n\x01\x08\x12\x03\x17\0(\n\t\
    \n\x02\x08\x01\x12\x03\x17\0(\n\x08\n\x01\x08\x12\x03\x18\0!\n\t\n\x02\
    \x08$\x12\x03\x18\0!\n\xa7\x14\n\x02\x04\0\x12\x04R\0^\x01\x1a\x9a\x14\
    \x20A\x20quaternion\x20is\x20defined\x20as\x20the\x20quotient\x20of\x20t\
    wo\x20directed\x20lines\x20in\x20a\n\x20three-dimensional\x20space\x20or\
    \x20equivalently\x20as\x20the\x20quotient\x20of\x20two\x20Euclidean\n\
    \x20vectors\x20(https://en.wikipedia.org/wiki/Quaternion).\n\n\x20Quater\
    nions\x20are\x20often\x20used\x20in\x20calculations\x20involving\x20thre\
    e-dimensional\n\x20rotations\x20(https://en.wikipedia.org/wiki/Quaternio\
    ns_and_spatial_rotation),\n\x20as\x20they\x20provide\x20greater\x20mathe\
    matical\x20robustness\x20by\x20avoiding\x20the\x20gimbal\x20lock\n\x20pr\
    oblems\x20that\x20can\x20be\x20encountered\x20when\x20using\x20Euler\x20\
    angles\n\x20(https://en.wikipedia.org/wiki/Gimbal_lock).\n\n\x20Quaterni\
    ons\x20are\x20generally\x20represented\x20in\x20this\x20form:\n\n\x20\
    \x20\x20\x20\x20w\x20+\x20xi\x20+\x20yj\x20+\x20zk\n\n\x20where\x20x,\
    \x20y,\x20z,\x20and\x20w\x20are\x20real\x20numbers,\x20and\x20i,\x20j,\
    \x20and\x20k\x20are\x20three\x20imaginary\n\x20numbers.\n\n\x20Our\x20na\
    ming\x20choice\x20`(x,\x20y,\x20z,\x20w)`\x20comes\x20from\x20the\x20des\
    ire\x20to\x20avoid\x20confusion\x20for\n\x20those\x20interested\x20in\
    \x20the\x20geometric\x20properties\x20of\x20the\x20quaternion\x20in\x20t\
    he\x203D\n\x20Cartesian\x20space.\x20Other\x20texts\x20often\x20use\x20a\
    lternative\x20names\x20or\x20subscripts,\x20such\n\x20as\x20`(a,\x20b,\
    \x20c,\x20d)`,\x20`(1,\x20i,\x20j,\x20k)`,\x20or\x20`(0,\x201,\x202,\x20\
    3)`,\x20which\x20are\x20perhaps\n\x20better\x20suited\x20for\x20mathemat\
    ical\x20interpretations.\n\n\x20To\x20avoid\x20any\x20confusion,\x20as\
    \x20well\x20as\x20to\x20maintain\x20compatibility\x20with\x20a\x20large\
    \n\x20number\x20of\x20software\x20libraries,\x20the\x20quaternions\x20re\
    presented\x20using\x20the\x20protocol\n\x20buffer\x20below\x20*must*\x20\
    follow\x20the\x20Hamilton\x20convention,\x20which\x20defines\x20`ij\x20=\
    \x20k`\n\x20(i.e.\x20a\x20right-handed\x20algebra),\x20and\x20therefore:\
    \n\n\x20\x20\x20\x20\x20i^2\x20=\x20j^2\x20=\x20k^2\x20=\x20ijk\x20=\x20\
    \xe2\x88\x921\n\x20\x20\x20\x20\x20ij\x20=\x20\xe2\x88\x92ji\x20=\x20k\n\
    \x20\x20\x20\x20\x20jk\x20=\x20\xe2\x88\x92kj\x20=\x20i\n\x20\x20\x20\
    \x20\x20ki\x20=\x20\xe2\x88\x92ik\x20=\x20j\n\n\x20Please\x20DO\x20NOT\
    \x20use\x20this\x20to\x20represent\x20quaternions\x20that\x20follow\x20t\
    he\x20JPL\n\x20convention,\x20or\x20any\x20of\x20the\x20other\x20quatern\
    ion\x20flavors\x20out\x20there.\n\n\x20Definitions:\n\n\x20\x20\x20-\x20\
    Quaternion\x20norm\x20(or\x20magnitude):\x20`sqrt(x^2\x20+\x20y^2\x20+\
    \x20z^2\x20+\x20w^2)`.\n\x20\x20\x20-\x20Unit\x20(or\x20normalized)\x20q\
    uaternion:\x20a\x20quaternion\x20whose\x20norm\x20is\x201.\n\x20\x20\x20\
    -\x20Pure\x20quaternion:\x20a\x20quaternion\x20whose\x20scalar\x20compon\
    ent\x20(`w`)\x20is\x200.\n\x20\x20\x20-\x20Rotation\x20quaternion:\x20a\
    \x20unit\x20quaternion\x20used\x20to\x20represent\x20rotation.\n\x20\x20\
    \x20-\x20Orientation\x20quaternion:\x20a\x20unit\x20quaternion\x20used\
    \x20to\x20represent\x20orientation.\n\n\x20A\x20quaternion\x20can\x20be\
    \x20normalized\x20by\x20dividing\x20it\x20by\x20its\x20norm.\x20The\x20r\
    esulting\n\x20quaternion\x20maintains\x20the\x20same\x20direction,\x20bu\
    t\x20has\x20a\x20norm\x20of\x201,\x20i.e.\x20it\x20moves\n\x20on\x20the\
    \x20unit\x20sphere.\x20This\x20is\x20generally\x20necessary\x20for\x20ro\
    tation\x20and\x20orientation\n\x20quaternions,\x20to\x20avoid\x20roundin\
    g\x20errors:\n\x20https://en.wikipedia.org/wiki/Rotation_formalisms_in_t\
    hree_dimensions\n\n\x20Note\x20that\x20`(x,\x20y,\x20z,\x20w)`\x20and\
    \x20`(-x,\x20-y,\x20-z,\x20-w)`\x20represent\x20the\x20same\x20rotation,\
    \n\x20but\x20normalization\x20would\x20be\x20even\x20more\x20useful,\x20\
    e.g.\x20for\x20comparison\x20purposes,\x20if\n\x20it\x20would\x20produce\
    \x20a\x20unique\x20representation.\x20It\x20is\x20thus\x20recommended\
    \x20that\x20`w`\x20be\n\x20kept\x20positive,\x20which\x20can\x20be\x20ac\
    hieved\x20by\x20changing\x20all\x20the\x20signs\x20when\x20`w`\x20is\n\
    \x20negative.\n\n\n\n\n\x03\x04\0\x01\x12\x03R\x08\x12\n\x1f\n\x04\x04\0\
    \x02\0\x12\x03T\x02\x0f\x1a\x12\x20The\x20x\x20component.\n\n\r\n\x05\
    \x04\0\x02\0\x04\x12\x04T\x02R\x14\n\x0c\n\x05\x04\0\x02\0\x05\x12\x03T\
    \x02\x08\n\x0c\n\x05\x04\0\x02\0\x01\x12\x03T\t\n\n\x0c\n\x05\x04\0\x02\
    \0\x03\x12\x03T\r\x0e\n\x1f\n\x04\x04\0\x02\x01\x12\x03W\x02\x0f\x1a\x12\
    \x20The\x20y\x20component.\n\n\r\n\x05\x04\0\x02\x01\x04\x12\x04W\x02T\
    \x0f\n\x0c\n\x05\x04\0\x02\x01\x05\x12\x03W\x02\x08\n\x0c\n\x05\x04\0\
    \x02\x01\x01\x12\x03W\t\n\n\x0c\n\x05\x04\0\x02\x01\x03\x12\x03W\r\x0e\n\
    \x1f\n\x04\x04\0\x02\x02\x12\x03Z\x02\x0f\x1a\x12\x20The\x20z\x20compone\
    nt.\n\n\r\n\x05\x04\0\x02\x02\x04\x12\x04Z\x02W\x0f\n\x0c\n\x05\x04\0\
    \x02\x02\x05\x12\x03Z\x02\x08\n\x0c\n\x05\x04\0\x02\x02\x01\x12\x03Z\t\n\
    \n\x0c\n\x05\x04\0\x02\x02\x03\x12\x03Z\r\x0e\n$\n\x04\x04\0\x02\x03\x12\
    \x03]\x02\x0f\x1a\x17\x20The\x20scalar\x20component.\n\n\r\n\x05\x04\0\
    \x02\x03\x04\x12\x04]\x02Z\x0f\n\x0c\n\x05\x04\0\x02\x03\x05\x12\x03]\
    \x02\x08\n\x0c\n\x05\x04\0\x02\x03\x01\x12\x03]\t\n\n\x0c\n\x05\x04\0\
    \x02\x03\x03\x12\x03]\r\x0eb\x06proto3\
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
