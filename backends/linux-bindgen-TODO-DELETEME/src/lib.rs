// As a quick test here, let's try dumping the generated bindings here...
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
include!(concat!(env!("OUT_DIR"), "/linux-bindgen.rs"));
