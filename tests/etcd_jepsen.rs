#[path = "../test_data/jepsen_loader.rs"]
mod jepsen_loader;
use jepsen_loader::load_jepsen_log;

fn check_jepsen(log_num: u32, correct: bool) {
    let events = load_jepsen_log(log_num);
    let res = porcupine::check_events(&events);
    assert_eq!(correct, res, "expected output {correct}, got output {res}")
}

macro_rules! etcd_test {
    ($log_num:expr, $expected:expr) => {
        paste::item! {
            #[test]
            fn [<etcd_test_ $log_num>]() {
               check_jepsen($log_num, $expected);
            }
        }
    };
}

etcd_test!(0, false);
etcd_test!(1, false);
etcd_test!(2, true);
etcd_test!(3, false);
etcd_test!(4, false);
etcd_test!(5, true);
etcd_test!(6, false);
etcd_test!(7, true);
etcd_test!(8, false);
etcd_test!(9, false);
etcd_test!(10, false);
etcd_test!(11, false);
etcd_test!(12, false);
etcd_test!(13, false);
etcd_test!(14, false);
etcd_test!(15, false);
etcd_test!(16, false);
etcd_test!(17, false);
etcd_test!(18, true);
etcd_test!(19, false);
etcd_test!(20, false);
etcd_test!(21, false);
etcd_test!(22, false);
etcd_test!(23, false);
etcd_test!(24, false);
etcd_test!(25, true);
etcd_test!(26, false);
etcd_test!(27, false);
etcd_test!(28, false);
etcd_test!(29, false);
etcd_test!(30, false);
etcd_test!(31, true);
etcd_test!(32, false);
etcd_test!(33, false);
etcd_test!(34, false);
etcd_test!(35, false);
etcd_test!(36, false);
etcd_test!(37, false);
etcd_test!(38, true);
etcd_test!(39, false);
etcd_test!(40, false);
etcd_test!(41, false);
etcd_test!(42, false);
etcd_test!(43, false);
etcd_test!(44, false);
etcd_test!(45, true);
etcd_test!(46, false);
etcd_test!(47, false);
etcd_test!(48, true);
etcd_test!(49, true);
etcd_test!(50, false);
etcd_test!(51, true);
etcd_test!(52, false);
etcd_test!(53, true);
etcd_test!(54, false);
etcd_test!(55, false);
etcd_test!(56, true);
etcd_test!(57, false);
etcd_test!(58, false);
etcd_test!(59, false);
etcd_test!(60, false);
etcd_test!(61, false);
etcd_test!(62, false);
etcd_test!(63, false);
etcd_test!(64, false);
etcd_test!(65, false);
etcd_test!(66, false);
etcd_test!(67, true);
etcd_test!(68, false);
etcd_test!(69, false);
etcd_test!(70, false);
etcd_test!(71, false);
etcd_test!(72, false);
etcd_test!(73, false);
etcd_test!(74, false);
etcd_test!(75, true);
etcd_test!(76, true);
etcd_test!(77, false);
etcd_test!(78, false);
etcd_test!(79, false);
etcd_test!(80, true);
etcd_test!(81, false);
etcd_test!(82, false);
etcd_test!(83, false);
etcd_test!(84, false);
etcd_test!(85, false);
etcd_test!(86, false);
etcd_test!(87, true);
etcd_test!(88, false);
etcd_test!(89, false);
etcd_test!(90, false);
etcd_test!(91, false);
etcd_test!(92, true);
etcd_test!(93, false);
etcd_test!(94, false);
// etcd cluster failed to start up in test 95
// etcd_test!(95, false);
etcd_test!(96, false);
etcd_test!(97, false);
etcd_test!(98, true);
etcd_test!(99, false);
etcd_test!(100, true);
etcd_test!(101, true);
etcd_test!(102, true);
