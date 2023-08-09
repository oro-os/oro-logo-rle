//! Reports the sizes of any enabled logo variants given the
//! configured features (e.g. with/without bz2 compression)

use oro_logo_rle::OroLogoData;

fn report_size<D: OroLogoData>(name: &str) {
	println!("{}: {}", name, D::framedata().len());
}

pub fn main() {
	#[cfg(feature = "oro-logo-1024")]
	report_size::<oro_logo_rle::OroLogo1024x1024>("OroLogo1024x1024");
	#[cfg(feature = "oro-logo-512")]
	report_size::<oro_logo_rle::OroLogo512x512>("OroLogo512x512");
	#[cfg(feature = "oro-logo-256")]
	report_size::<oro_logo_rle::OroLogo256x256>("OroLogo256x256");
	#[cfg(feature = "oro-logo-64")]
	report_size::<oro_logo_rle::OroLogo64x64>("OroLogo64x64");
	#[cfg(feature = "oro-logo-32")]
	report_size::<oro_logo_rle::OroLogo32x32>("OroLogo32x32");
}
