use std::{env::current_dir, fs::File, io::prelude::Write};

use crate::world_gen::erosion::{EROSION_DISPATCH_SIZE, EROSION_WORKGROUP_SIZE, MAX_EROSION_STEPS};

macro_rules! constant_to_wgsl {
    ($constant:ident) => {
        &format!("pub const {:} = {:};\n", stringify!($constant), $constant)
    };
}

pub fn create_shader_constants() {
    let mut path = current_dir().unwrap();
    path.push("assets");
    path.push("shaders");
    path.push("constants.wgsl");
    //Create File for shader constants
    let mut file = File::create(path).unwrap();
    let mut text = String::new();
    //Define the module name
    text.push_str("#define_import_path shader_constants\n");
    //Define the constants
    text.push_str(constant_to_wgsl!(EROSION_WORKGROUP_SIZE));
    text.push_str(constant_to_wgsl!(EROSION_DISPATCH_SIZE));
    text.push_str(constant_to_wgsl!(MAX_EROSION_STEPS));
    text.push_str(&format!("pub const PI = {:};\n", std::f64::consts::PI));

    //Write the text to the file
    file.write_all(text.as_bytes()).unwrap();
}
