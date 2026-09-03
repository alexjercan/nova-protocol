//! Fixed landmarks and asteroid layout shared by both First Shift map benches.

use nova_protocol::prelude::*;

pub const CARRIER_POS: Meters3 = Meters3::new(-1_000.0, 0.0, 2_500.0);
pub const INSPECTION_POS: Meters3 = Meters3::new(-4_500.0, -400.0, -6_500.0);
pub const CONCEALMENT_POS: Meters3 = Meters3::new(4_500.0, 300.0, -6_500.0);
pub const INSPECTION_RADIUS: Meters = Meters(200.0);
pub const CONCEALMENT_RADIUS: Meters = Meters(500.0);

/// A broad plate of small rocks between the carrier and both planetoids. The
/// former banana's narrow tail is gone; its curved bowl is filled so the cutter
/// has several slalom lines while the carrier remains too large to enter.
pub const SALVAGE_ROCKS: [(Meters3, Meters); 40] = [
    (Meters3::new(400.0, 220.0, -1_200.0), Meters(32.0)),
    (Meters3::new(1_000.0, -260.0, -1_000.0), Meters(22.0)),
    (Meters3::new(1_700.0, 320.0, -1_050.0), Meters(35.0)),
    (Meters3::new(2_300.0, -220.0, -1_250.0), Meters(26.0)),
    (Meters3::new(2_800.0, 240.0, -1_700.0), Meters(30.0)),
    (Meters3::new(3_000.0, -280.0, -2_300.0), Meters(20.0)),
    (Meters3::new(3_050.0, 300.0, -2_900.0), Meters(34.0)),
    (Meters3::new(2_900.0, -220.0, -3_500.0), Meters(24.0)),
    (Meters3::new(2_600.0, 280.0, -4_100.0), Meters(30.0)),
    (Meters3::new(2_100.0, -260.0, -4_500.0), Meters(22.0)),
    (Meters3::new(1_400.0, 220.0, -4_700.0), Meters(35.0)),
    (Meters3::new(700.0, -240.0, -4_500.0), Meters(20.0)),
    (Meters3::new(200.0, 300.0, -4_100.0), Meters(28.0)),
    (Meters3::new(-100.0, -200.0, -3_500.0), Meters(25.0)),
    (Meters3::new(-200.0, 260.0, -2_800.0), Meters(35.0)),
    (Meters3::new(-50.0, -220.0, -2_100.0), Meters(24.0)),
    (Meters3::new(600.0, 300.0, -1_800.0), Meters(30.0)),
    (Meters3::new(1_300.0, -250.0, -1_600.0), Meters(18.0)),
    (Meters3::new(2_000.0, 240.0, -1_750.0), Meters(28.0)),
    (Meters3::new(2_500.0, -300.0, -2_200.0), Meters(22.0)),
    (Meters3::new(2_600.0, 260.0, -2_900.0), Meters(31.0)),
    (Meters3::new(2_350.0, -220.0, -3_500.0), Meters(19.0)),
    (Meters3::new(1_800.0, 300.0, -3_950.0), Meters(26.0)),
    (Meters3::new(1_100.0, -260.0, -4_000.0), Meters(22.0)),
    (Meters3::new(500.0, 260.0, -3_600.0), Meters(30.0)),
    (Meters3::new(300.0, -280.0, -3_000.0), Meters(20.0)),
    (Meters3::new(450.0, 320.0, -2_400.0), Meters(32.0)),
    (Meters3::new(1_200.0, -300.0, -2_700.0), Meters(24.0)),
    // Fill the former bowl so the field reads as a plate, not a ring.
    (Meters3::new(850.0, 80.0, -2_100.0), Meters(24.0)),
    (Meters3::new(1_450.0, -60.0, -2_100.0), Meters(20.0)),
    (Meters3::new(1_900.0, 100.0, -2_300.0), Meters(26.0)),
    (Meters3::new(750.0, -100.0, -2_700.0), Meters(18.0)),
    (Meters3::new(1_550.0, 80.0, -2_700.0), Meters(28.0)),
    (Meters3::new(2_050.0, -80.0, -2_850.0), Meters(20.0)),
    (Meters3::new(800.0, 120.0, -3_250.0), Meters(22.0)),
    (Meters3::new(1_400.0, -100.0, -3_350.0), Meters(20.0)),
    (Meters3::new(1_950.0, 100.0, -3_300.0), Meters(24.0)),
    (Meters3::new(1_050.0, 60.0, -2_400.0), Meters(18.0)),
    (Meters3::new(1_650.0, -120.0, -2_450.0), Meters(22.0)),
    (Meters3::new(1_150.0, 120.0, -3_000.0), Meters(19.0)),
];

pub const AMBIENT_ROCKS: [(Meters3, Meters); 20] = [
    (Meters3::new(-6_000.0, 1_000.0, -1_000.0), Meters(55.0)),
    (Meters3::new(-4_200.0, -900.0, -2_500.0), Meters(40.0)),
    (Meters3::new(-2_500.0, 1_300.0, -1_500.0), Meters(65.0)),
    (Meters3::new(-1_800.0, -1_100.0, -3_800.0), Meters(35.0)),
    (Meters3::new(500.0, 1_000.0, -2_500.0), Meters(45.0)),
    (Meters3::new(1_600.0, -900.0, -1_200.0), Meters(60.0)),
    (Meters3::new(3_200.0, 1_300.0, -2_700.0), Meters(38.0)),
    (Meters3::new(5_200.0, -1_000.0, -2_000.0), Meters(70.0)),
    (Meters3::new(7_000.0, 700.0, -3_500.0), Meters(42.0)),
    (Meters3::new(8_000.0, -1_200.0, -5_000.0), Meters(55.0)),
    (Meters3::new(-8_000.0, 1_500.0, -3_500.0), Meters(48.0)),
    (Meters3::new(-8_500.0, -1_000.0, -8_000.0), Meters(75.0)),
    (Meters3::new(-6_500.0, 1_800.0, -10_000.0), Meters(44.0)),
    (Meters3::new(-2_500.0, -1_500.0, -10_000.0), Meters(62.0)),
    (Meters3::new(1_000.0, 1_700.0, -9_500.0), Meters(40.0)),
    (Meters3::new(3_500.0, -1_400.0, -10_000.0), Meters(68.0)),
    (Meters3::new(6_500.0, 1_600.0, -9_500.0), Meters(50.0)),
    (Meters3::new(8_500.0, -800.0, -8_000.0), Meters(72.0)),
    (Meters3::new(9_000.0, 1_200.0, -6_000.0), Meters(46.0)),
    (Meters3::new(7_000.0, -1_600.0, -7_000.0), Meters(58.0)),
];
