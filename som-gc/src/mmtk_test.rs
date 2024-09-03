// use mmtk;
// 
// #[no_mangle]
// pub fn mmtk_init(heap_size: usize) {
//     let mut builder = mmtk::MMTKBuilder::new();
// 
//     // Set option by value using extern "C" wrapper.
//     let success = mmtk_set_fixed_heap_size(&mut builder, heap_size);
//     assert!(success);
// 
//     let ok = builder
//         .options
//         .plan
//         .set(mmtk::util::options::PlanSelector::NoGC);
//     if !ok {
//         panic!("invalid plan selector");
//     }
// 
//     // Create MMTK instance.
//     let mmtk = memory_manager::mmtk_init::<DummyVM>(&builder);
// 
//     // Set SINGLETON to the instance.
//     GC.set(mmtk).unwrap_or_else(|_| {
//         panic!("Failed to set SINGLETON");
//     });
// }