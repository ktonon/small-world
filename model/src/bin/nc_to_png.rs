use small_world_model::convert_nc_to_png;
use std::error::Error;
use std::path::Path;

fn main() -> Result<(), Box<dyn Error>> {
    convert_nc_to_png(Path::new("../data/age.2020.1.GTS2012.1m.classic.nc"))
}
