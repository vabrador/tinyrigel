
use core::ffi;
use std::ffi::c_void;
use std::{convert::TryFrom, ffi::OsString, os::windows::prelude::OsStringExt, ptr};

use packed_struct::prelude::*;
use windows::*;
use windows_bindings::Windows::Win32::System::Com::*;
use windows_bindings::{
  Windows::Devices::Enumeration as WinDevEnum,
  Windows::Devices::Custom as WinDevCustom,
  Windows::Foundation as WinFoundation,
  Windows::Media::Capture as WinMediaCap,
  Windows::Media::MediaProperties as WinMediaProps,
  Windows::Storage::Streams as WinStorageStreams,

  Windows::Win32::Devices::Usb,
  Windows::Win32::Devices::DeviceAndDriverInstallation as Win32Device,
  Windows::Win32::Foundation as Win32Foundation,
  Windows::Win32::System::Diagnostics::Debug as Win32Debug,
  Windows::Win32::Storage::FileSystem as Win32FileSystem,
  Windows::Win32::System::Com as Win32Com,
  Windows::Win32::System::OleAutomation as Win32OleAuto,
  Windows::Win32::Graphics::DirectShow as Win32DirectShow,
  Windows::Win32::Media::Audio::CoreAudio as Win32CoreAudio,
  Windows::Win32::System::SystemServices as Win32SystemServices,
  Windows::Win32::System::Threading as Win32SysThreading,
};

use windows_bindings::Windows::Win32::System::WinRT::IMemoryBufferByteAccess;
unsafe fn as_mut_slice(buffer: &WinFoundation::IMemoryBufferReference) -> Result<&mut [u8]> {
  let interop = buffer.cast::<IMemoryBufferByteAccess>()?;
  let mut data = std::ptr::null_mut();
  let mut len = 0;

  interop.GetBuffer(&mut data, &mut len)?;
  Ok(std::slice::from_raw_parts_mut(data, len as _))
}

type BoxResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

// This returns e.g. my logitech USB camera.
// ImagingDeviceClassGUID: `6BDD1FC6-810F-11D0-BEC7-08002BE2092F`
const imaging_device_class_guid: windows::Guid = windows::Guid::from_values(
  0x6BDD1FC6, 0x810F, 0x11D0, [0xBE, 0xC7, 0x08, 0x00, 0x2B, 0xE2, 0x09, 0x2F], 
);
// This returns (surprise) all the connected USB cameras! But not imaging devices, which is a different thing apparently.
// CameraDeviceClassGUID: `CA3E7AB9-B4C3-4AE6-8251-579EF933890F`
const camera_device_class_guid: windows::Guid = windows::Guid::from_values(
  0xCA3E7AB9, 0xB4C3, 0x4AE6, [0x82, 0x51, 0x57, 0x9E, 0xF9, 0x33, 0x89, 0x0F]
);
// USBSerialDeviceClassGUID: `4d36e978-e325-11ce-bfc1-08002be10318`
// On my machine these appear to not be related to any Leap devices.
const usb_serial_device_class_guid: windows::Guid = windows::Guid::from_values(
  0x4D36E978, 0xE325, 0x11CE, [0xBF, 0xC1, 0x08, 0x00, 0x2B, 0xE1, 0x03, 0x18]
);
// LeapUSBDeviceInterfaceGUID: `2B1A1B59-D3AC-4841-AA28-4819A66D7CB9`
// This is an interface GUID, not a device class GUID.
const leap_usb_device_interface_guid: windows::Guid = windows::Guid::from_values(
  0x2B1A1B59, 0xD3AC, 0x4841, [0xAA, 0x28, 0x48, 0x19, 0xA6, 0x6D, 0x7C, 0xB9]
);
// TeensyUSBControllerGUID `40908837-F958-4025-B3BF-0A0FD990DDF0`

// Definition of the strobe GUID in standard GUID layout
// leap_xu.h
// #define LEAP_XU_GUID {0x9ADA33BC, 0x8B9B, 0x4B7F, {0xA9, 0xD1, 0xF3, 0xBC, 0x8C, 0x63, 0x90, 0x29}}
pub const LEAP_XU_GUID: windows::Guid = windows::Guid::from_values(
  0x9ADA33BC, 0x8B9B, 0x4B7F, [0xA9, 0xD1, 0xF3, 0xBC, 0x8C, 0x63, 0x90, 0x29]
);

/// This interface for some reason is not exposed in the windows-rs bindings.
const CLSID_GUID_IKsObject: windows::Guid = windows::Guid::from_values(
  // 423c13a2-2070-11d0-9ef7-00aa00a216a1
  0x423C13A2, 0x2070, 0x11D0, [0x9E, 0xF7, 0x00, 0xAA, 0x00, 0xA2, 0x16, 0xA1]
);

#[derive(Debug)]
struct TestError { details: String }
impl TestError {
  fn new(msg: &str) -> TestError { TestError { details: msg.to_string() }}
  fn err<T>(msg: &str) -> std::result::Result<T, Box<dyn std::error::Error>> {
    Err(Box::new(TestError::new(msg)))
  }
}
use std::fmt;
impl fmt::Display for TestError { fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
  write!(f, "{}", self.details)
}}
impl std::error::Error for TestError { fn description(&self) -> &str { &self.details }}

/// DirectShow control node index or ID, gotten by querying topology information in ID order.
struct NodeID(u32);


// // // #[implement(Windows::Win32::Graphics::DirectShow::IKsNodeControl)]
// // // pub struct LeapXU {
// // //   // IKsNodeControl
// // //   dwnodeid: u32,
// // //   ikscontrol: Win32DirectShow::IKsControl,
// // // }
// // // #[allow(non_snake_case)]
// // // impl LeapXU {
// // //   // IKsNodeControl
// // //   // --------------
// // //   // https://docs.microsoft.com/en-us/windows-hardware/drivers/stream/sample-extension-unit-plug-in-dll
// // //   unsafe fn SetNodeId(&mut self, dwnodeid: u32) -> Result<()> {
// // //     self.dwnodeid = dwnodeid;
// // //     Ok(())
// // //   }
// // //   unsafe fn SetKsControl(&mut self, pkscontrol: *mut c_void) -> Result<()> {
// // //     if pkscontrol.is_null() {
// // //       return Err(windows::Error::new(Win32Foundation::E_POINTER, "NULL was passed incorrectly for a pointer value." ));
// // //     }

// // //     let ikscontrol: Win32DirectShow::IKsControl = std::mem::transmute(pkscontrol);
// // //     if !std::mem::transmute::<&Win32DirectShow::IKsControl, *mut c_void>(&self.ikscontrol).is_null() {
// // //       // Here the equivalent C code would release the old IKsControl pointer...
// // //       // I think we can just not do this because Rust will drop the old IKsControl when we assign the new one?
// // //     }
    
// // //     let cast_res = ikscontrol.cast::<Win32DirectShow::IKsControl>();
// // //     if cast_res.is_err() { return Err(cast_res.unwrap_err()); }
// // //     else {
// // //       self.ikscontrol = ikscontrol;
// // //     }

// // //     Ok(())
// // //   }
// // // }


// // // use windows::*;
// // // use windows_bindings::Windows::Win32::Foundation::*;
// // // use windows_bindings::Windows::Win32::System::Com::*;
// // // use windows_bindings::Windows;
// // // #[implement(Windows::Win32::System::Com::IDataObject)]
// // // #[derive(Default)]
// // // #[allow(non_snake_case)]
// // // struct TestDataObject {
// // //     GetData: bool,
// // //     GetDataHere: bool,
// // //     QueryGetData: bool,
// // //     GetCanonicalFormatEtc: bool,
// // //     SetData: bool,
// // //     EnumFormatEtc: bool,
// // //     DAdvise: bool,
// // //     DUnadvise: bool,
// // //     EnumDAdvise: bool,
// // // }
// // // #[allow(non_snake_case)]
// // // impl TestDataObject {
// // //     fn GetData(&mut self, _: *const FORMATETC) -> Result<STGMEDIUM> {
// // //         self.GetData = true;
// // //         Ok(STGMEDIUM {
// // //             tymed: 0,
// // //             Anonymous: STGMEDIUM_0 {
// // //                 pstg: std::ptr::null_mut(),
// // //             },
// // //             pUnkForRelease: None,
// // //         })
// // //     }

// // //     fn GetDataHere(&mut self, _: *const FORMATETC, _: *mut STGMEDIUM) -> Result<()> {
// // //         self.GetDataHere = true;
// // //         Ok(())
// // //     }

// // //     fn QueryGetData(&mut self, _: *const FORMATETC) -> Result<()> {
// // //         self.QueryGetData = true;
// // //         Ok(())
// // //     }

// // //     fn GetCanonicalFormatEtc(&mut self, _: *const FORMATETC) -> Result<FORMATETC> {
// // //         self.GetCanonicalFormatEtc = true;
// // //         Ok(FORMATETC::default())
// // //     }

// // //     fn SetData(&mut self, _: *const FORMATETC, _: *const STGMEDIUM, _: BOOL) -> Result<()> {
// // //         self.SetData = true;
// // //         Ok(())
// // //     }

// // //     fn EnumFormatEtc(&mut self, _: u32) -> Result<IEnumFORMATETC> {
// // //         self.EnumFormatEtc = true;
// // //         Err(Error::OK)
// // //     }

// // //     fn DAdvise(&mut self, _: *const FORMATETC, _: u32, _: &Option<IAdviseSink>) -> Result<u32> {
// // //         self.DAdvise = true;
// // //         Ok(0)
// // //     }

// // //     fn DUnadvise(&mut self, _: u32) -> Result<()> {
// // //         self.DUnadvise = true;
// // //         Ok(())
// // //     }

// // //     fn EnumDAdvise(&mut self) -> Result<IEnumSTATDATA> {
// // //         self.EnumDAdvise = true;
// // //         Err(Error::OK)
// // //     }
// // // }

// // // #[test]
// // // fn test_implement() -> Result<()> {
// // //     unsafe {
// // //         let d: IDataObject = TestDataObject::default().into();
// // //         d.GetData(std::ptr::null_mut())?;
// // //         d.GetDataHere(std::ptr::null_mut(), std::ptr::null_mut())?;
// // //         d.QueryGetData(std::ptr::null_mut())?;
// // //         d.GetCanonicalFormatEtc(std::ptr::null_mut())?;
// // //         d.SetData(std::ptr::null_mut(), std::ptr::null_mut(), false)?;
// // //         let _ = d.EnumFormatEtc(0);
// // //         d.DAdvise(std::ptr::null_mut(), 0, None)?;
// // //         d.DUnadvise(0)?;
// // //         let _ = d.EnumDAdvise();

// // //         let i = TestDataObject::to_impl(&d);
// // //         assert!(i.GetData);
// // //         assert!(i.GetDataHere);
// // //         assert!(i.QueryGetData);
// // //         assert!(i.GetCanonicalFormatEtc);
// // //         assert!(i.SetData);
// // //         assert!(i.EnumFormatEtc);
// // //         assert!(i.DAdvise);
// // //         assert!(i.DUnadvise);
// // //         assert!(i.EnumDAdvise);

// // //         Ok(())
// // //     }
// // // }


// This is EXTREMELY important in matching Platform's method to interact with the Rigel's extension unit interface.
//
// We can't ACTUALLY use DirectShow's IKsControl KsProperty method to manipulate properties on our Leap firmware Extension Unit because 
com::interfaces! {
  #[uuid("00000000-0000-0000-C000-000000000046")]
    pub unsafe interface IUnknown {
        fn QueryInterface(
            &self,
            riid: *const com::IID,
            ppv: *mut *mut c_void
        ) -> HRESULT;
        fn AddRef(&self) -> u32;
        fn Release(&self) -> u32;
    }

  #[uuid("423C13A2-2070-11D0-9EF7-00AA00A216A1")]
  pub unsafe interface IKsObject: IUnknown {
    fn KsGetObjectHandle(&self) -> Win32Foundation::HANDLE;
  }
}




// RUN THIS TEST VIA: `cargo test enumerate_and_open_leap_xu -- --nocapture`
#[test]
fn enumerate_and_open_leap_xu() -> std::result::Result<(), Box<dyn std::error::Error>> {

  // We naively spin to wait out asynchronous requests. This sets the timeout in "loops" to wait for them (TODO: this SHOULD be a time duration!)
  let timeout = 20_000_000u64;

  // Use latest Windows Devices API to retrieve device PNPIDs.

  let dev_infos = naive_wait_and_get_op_results(&WinDevEnum::DeviceInformation::FindAllAsyncDeviceClass(WinDevEnum::DeviceClass::VideoCapture)?, timeout)?;

  let leap_dev = dev_infos.into_iter().find(|dev_info| is_leap_device(dev_info).unwrap_or(false));
  if leap_dev.is_none() { return TestError::err("No leap device detected! Is your Rigel or Peripheral connected via USB?"); }
  let leap_dev = leap_dev.unwrap();

  // The device filter we're looking for will contain a Leap vendor VID and known product PID.
  let leap_dev_id = leap_dev.Id()?.to_string_lossy().to_uppercase();
  println!("The first Leap device enumerated was: {}\nWe will attempt to open the device via DirectShow for Extension Unit access.", leap_dev_id);
  let leap_dev_hdw_id = get_device_prop_hardware_id(&leap_dev)?;
  println!("Hardware ID of the device is: {}", leap_dev_hdw_id);

  // Let's enumerate properties again.
  let props = leap_dev.Properties()?;
  let props_iter = props.First().expect("Iter fail");
  loop {
    let iter_kvp = props_iter.Current().expect("Current fail");
    let iter_key_res = iter_kvp.Key();
    if iter_key_res.is_ok() {
      println!("\t\t Key: {}", iter_key_res.unwrap());
    }
    let iter_value_res = iter_kvp.Value();
    if iter_value_res.is_ok() {
      let value = iter_value_res.unwrap();
      println!("\t\t Value: {:?}", value);
      let val_str_maybe = HSTRING::try_from(&value);
      if val_str_maybe.is_ok() {
        println!("\t\t Value str: {:?}", val_str_maybe.unwrap());
      }
      let bool_maybe = bool::try_from(&value);
      if bool_maybe.is_ok() {
        println!("\t\t Value bool: {:?}", bool_maybe.unwrap());
      }
    }

    let has_next = props_iter.MoveNext().expect("MoveNext fail");
    if !has_next { break; }
  }

  // Note: The above might not be necessary if we're just going to match over VID/PID...

  // Enumerate via DirectShow and find the DShow filter that matches the device ID we found. This is how we'll access Extension Units.
  
  // See platform/uuids.h for a list of GUIDs.
  // CLSID_SystemDeviceEnum
  let clsid_system_device_enum = windows::Guid::from_values(
    0x62BE5D10,0x60EB,0x11d0,[0xBD,0x3B,0x00,0xA0,0xC9,0x11,0xCE,0x86]
  );
  let create_dev_enum: Win32DirectShow::ICreateDevEnum = unsafe {
    Win32Com::CoCreateInstance(
      &clsid_system_device_enum,
      None,
      Win32Com::CLSCTX_INPROC_SERVER
    )?
  };

  let video_input_device_category_guid = windows::Guid::from_values(
    0x860BB310,0x5D01,0x11d0,[0xBD,0x3B,0x00,0xA0,0xC9,0x11,0xCE,0x86]
  );
  let mut dev_moniker: Option<Win32Com::IMoniker> = None;
  let mut dev_filter_ptr: *mut ffi::c_void = std::ptr::null_mut();
  let mut dev_filter: Option<Win32DirectShow::IBaseFilter> = None;
  unsafe {
    let dev_enum_moniker_ptr: *mut Option<Win32Com::IEnumMoniker> = &mut Option::None;
    let res = create_dev_enum.CreateClassEnumerator(
      &video_input_device_category_guid,
      dev_enum_moniker_ptr,
      0 as _
    );
    if res.is_err() {
      println!("Error enumerating video input device category: {:?}", res.err());
      print_last_error();
      return Ok(());
    }
    // dev_enum_moniker_ptr now valid.
    
    let device_filter: Option<u32> = None;
    
    let num_fetched_ptr = &mut 0u32;
    let dev_moniker_ptr: *mut Option<Win32Com::IMoniker> = &mut dev_moniker;
    loop {
      if let Some(_) = device_filter { break; }
      
      let res = (*dev_enum_moniker_ptr).as_ref().unwrap().Next(1, dev_moniker_ptr, num_fetched_ptr);
      println!("num fetched... {}", *num_fetched_ptr);
      if res.is_err() {
        println!("Error advancing moniker enumerator: ");
        print_last_error();
        break;
      } else {
        println!("Got an advance, have a dev_moniker.");
      }
      // Finished when we don't get anything more back.
      if *num_fetched_ptr == 0 { break; }

      // // // // let mut prop_bag_ptr: Option<Win32OleAuto::IPropertyBag> = None;
      // // // let prop_bag: Win32OleAuto::IPropertyBag = dev_monikor.as_ref().unwrap().BindToStorage(None, None)?;

      let dev_moniker = (*dev_moniker_ptr).as_ref().unwrap();
      let dev_display_name_pwstr = dev_moniker.GetDisplayName(None, None)?;
      let dev_display_name_ptr = dev_display_name_pwstr.0;
      let dev_display_name = widestring::U16CString::from_ptr_str(dev_display_name_ptr).to_os_string();
      println!("Got dev_moniker display name: {}", dev_display_name.to_string_lossy());

      if contains_leap_vendor_id(&dev_display_name.into_string().unwrap()) {
        println!("This is a Leap device.");

        println!("Attempting CreateFile...");
        let dev_handle = Win32FileSystem::CreateFileW(
          dev_display_name_pwstr,
          Win32FileSystem::FILE_GENERIC_READ | Win32FileSystem::FILE_GENERIC_WRITE,
          Win32FileSystem::FILE_SHARE_READ | Win32FileSystem::FILE_SHARE_WRITE,
          0 as _ /* nullptr */,
          Win32FileSystem::OPEN_EXISTING,
          Win32FileSystem::FILE_ATTRIBUTE_NORMAL | Win32FileSystem::FILE_FLAG_OVERLAPPED,
          Win32Foundation::HANDLE::NULL,
        );
        if dev_handle.is_invalid() {
          println!("Tried to create a file handle for I/O with the Leap Motion device, but it failed.");
          print_last_error();
        } else {
          println!("OPENED LEAP DEVICE. CLOSING IT NOW...");
          let closed = Win32Foundation::CloseHandle(dev_handle);
          if !closed.as_bool() { println!("ERROR CLOSING."); print_last_error(); }
          else { println!("OK, closed."); }
        }

        break;
      }
    }
    if (*dev_moniker_ptr).is_none() {
      return TestError::err("Failed to find a Leap device in DirectShow enumeration. Is your Rigel or Peripheral plugged in?");
    } else {
      println!("Found a Leap device via DirectShow.");
    }

    // Try to bind to the Leap device. This will tell us whether the device is ready.
    println!("\nTrying to bind filter..");

    let IBaseFilter_GUID = windows::Guid::from_values(
      0x56a86899, 0x0ad4, 0x11ce, [0xb0, 0x3a, 0x00, 0x20, 0xaf, 0x0b, 0xa7, 0x70]
    );

    println!("Before bind to object: {:p}", dev_filter_ptr);
    let ret_val = dev_moniker.as_ref().unwrap().BindToObject(None, None, &Win32DirectShow::IBaseFilter::IID, &mut dev_filter_ptr);
    println!("After bind to object: {:p}", dev_filter_ptr);
    
    if ret_val.is_err() {
      println!("Found a Leap device, but it could not be bound as a filter, so it was not ready!");
      return TestError::err("Leap device not ready");
    } else {
      println!("OK, device was ready");
    }

    println!("Trying to reinterpret the ptr as the IBaseFilter...");
    let filter: Win32DirectShow::IBaseFilter = std::mem::transmute(dev_filter_ptr);
    dev_filter = Some(filter)

    // println!("Trying to deref for dev_filter... {:p}", dev_filter_ptr);
    // let foo = &*dev_filter_ptr;
    // println!("Now casting...");
    // dev_filter = Some(foo.cast::<Win32DirectShow::IBaseFilter>());
  };

  // dev_filter is the KSObject for the Leap device. This is the object we can use to generate IOCTLs to send XU (Extension Unit) commands.
  // We need to get the available control objects that are contained in the topology of the KSObject node for the Leap device.
  // Nick theorizing: This will allow us to ... compare them against known GUIDs for XUs to send our XU commands to the right XU unit exposed by the device...?

  // let guid_ext_code = 9ADA33BC-8B9B-4B7F-A9D1-F3BC8C639029;
  // LEAP_XU_GUID: windows::Guid = windows::Guid::from_values(
  //   0x9ADA33BC, 0x8B9B, 0x4B7F, [0xA9, 0xD1, 0xF3, 0xBC, 0x8C, 0x63, 0x90, 0x29]
  // );
  // let other_guid = DD880F8A-1CBA-4954-8A25-F7875967F0F7
  let OMNIVISION_XU_GUID = windows::Guid::from_values(
    0xDD880F8A, 0x1CBA, 0x4954, [0x8A, 0x25, 0xF7, 0x87, 0x59, 0x67, 0xF0, 0xF7]
  );
  // // // let mut ext_prop = Win32CoreAudio::KSIDENTIFIER {
  // // //   Anonymous: Win32CoreAudio::KSIDENTIFIER_0 {
  // // //     Anonymous: Win32CoreAudio::KSIDENTIFIER_0_0 {
  // // //       Set: LEAP_XU_GUID,
  // // //       // Set: OTHER_XU_GUID,
  // // //       // Id: Win32CoreAudio::KSPROPERTY_EXTENSION_UNIT_INFO.0 as u32,
  // // //       Id: 0,
  // // //       // Flags: Win32CoreAudio::KSPROPERTY_TYPE_SETSUPPORT | Win32CoreAudio::KSPROPERTY_TYPE_TOPOLOGY,
  // // //       Flags: 0x1000_0100,
  // // //     }
  // // //   }
  // // // };

  // Let's try to get an IKsPropertySet from the IBaseFilter.
  // // println!("===== IBaseFilter -> IKsPropertySet =====");
  // // println!("Attempting cast..");
  // // let ikspropertyset = 
  // // println!("");

  // We need another IFilterBase reference to turn into IKsTopologyInfo.
  // // // println!("Trying to get filter_dup");
  // // // let filter_dup: Win32DirectShow::IBaseFilter = unsafe { dev_moniker.as_ref().unwrap().BindToObject(None, None)? };
  println!("Trying to cast IBaseFilter into IKsTopologyInfo...");
  let filter_topo: Win32DirectShow::IKsTopologyInfo = dev_filter.as_ref().unwrap().cast()?;
  let node_count = unsafe { filter_topo.get_NumNodes()? };
  let mut device_xu_control_nodes = vec![];
  println!("Enumerating {} nodes in IKsTopologyInfo..", node_count);
  for n in 0..node_count {
    let node_type = unsafe { filter_topo.get_NodeType(n)? };
    println!("\nNode type {} GUID is {:?}", n, node_type);

    println!("Trying to get control node n = {}", n);
    let maybe_control_node: Result<Win32DirectShow::IKsControl> = unsafe {
      let mut control_node_ptr = ptr::null_mut();
      println!("control_node_ptr before CreateNodeInstance: {:p}", control_node_ptr);
      filter_topo.CreateNodeInstance(n, &Win32DirectShow::IKsControl::IID, &mut control_node_ptr)?;
      println!("control_node_ptr after CreateNodeInstance: {:p}", control_node_ptr);
      Ok(std::mem::transmute(control_node_ptr))
    };
    if maybe_control_node.is_err() { 
      println!("Failed to create an IKsControl for a node, skipping it.."); continue;
    }

    let ikscontrol = maybe_control_node.unwrap();

    println!("-- Attempting to go from IKsControl -> IUnknown -> IKsObject raw --");
    let control_iunknown = ikscontrol.cast::<windows::IUnknown>()?;
    let GUID_IID_IKsObject = windows::Guid::from_values(
      0x423c13a2, 0x2070, 0x11d0, [0x9e, 0xf7, 0x00, 0xaa, 0x00, 0xa2, 0x16, 0xa1]
    );
    let mut control_iksobject = ptr::null_mut();
    println!("control_iksobject before QueryInterface: {:p}", control_iksobject);
    let get_control_iksobject_res = unsafe { control_iunknown.query(&GUID_IID_IKsObject, &mut control_iksobject) };
    if get_control_iksobject_res.is_err() {
      println!("Error: {:?}", get_control_iksobject_res);
    }
    println!("control_iksobject after QueryInterface: {:p}", &control_iksobject);
    
    let custom_specified_iksobject: IKsObject = unsafe { std::mem::transmute(control_iksobject) };

    // let mut bytes_returned = 0u32;
    // let res = unsafe { ikscontrol.KsProperty(
    //   &mut ext_prop,
    //   32 /* not 24... ......? */,
    //   ptr::null_mut(),
    //   0,
    //   &mut bytes_returned
    // )};
    // if res.is_err() { println!("Node {}... Property request returned error: {:?}", n, res.err()); }

    if node_type != Win32CoreAudio::KSNODETYPE_DEV_SPECIFIC {
      println!("Skipping a non-device-specific node.."); continue; }

    device_xu_control_nodes.push((NodeID(n), custom_specified_iksobject));
  }
  println!("\nFinished scanning.");
  println!("OK, scanned {} device-specific nodes into device_xu_control_nodes.", device_xu_control_nodes.len());

  // From here, referencing PLatform UVCTransportDShow:
  // - m_DeviceFilter is dev_filter
  // - m_ksObjs is device_xu_control_nodes

  // Let's try to route a GetCur for calibration data.

  // LEAP_XU_CALIBRATION_DATA: u32 = 0x19; see: leap_xu.h
  let leap_xu_selector_calib_data = 0x19;
  use crate::calib::SerializedCalibration;
  let mut calib_data = SerializedCalibration::empty_calibration();
  let calib_data_ptr = &mut calib_data as *mut SerializedCalibration;
  let calib_data_len = std::mem::size_of::<SerializedCalibration>();

  // We need the node control for the correct Extension Unit. We don't know if it's the first one or the second one.
  for (node_id, iksobject) in &device_xu_control_nodes {
    let query_for_guid = LEAP_XU_GUID;
    let mut xu_ksp_node = construct_ksp_node_for_xu_query(
      LEAP_XU_GUID,
      Win32CoreAudio::KSPROPERTY_EXTENSION_UNIT_INFO.0 as _,
      Win32CoreAudio::KSPROPERTY_TYPE_SETSUPPORT | Win32CoreAudio::KSPROPERTY_TYPE_TOPOLOGY,
      node_id.0
    );
    // let xu_ksp_node_ptr = &xu_ksp_node as *const Win32CoreAudio::KSP_NODE;

    // Try sending DeviceIoControl.
    let dev_io_handle = unsafe { iksobject.KsGetObjectHandle() };
    if dev_io_handle.is_invalid() {
      println!("The device_io_handle was invalid. :(");
      print_last_error();
    } else {

    }
    let res_bytes_returned = send_device_io_control_kspropxu(
      dev_io_handle,
      xu_ksp_node,
      ptr::null_mut(), // lpoutbuffer -> nullptr to get info
      0, // output size is zero, we're just getting info
    );
    if res_bytes_returned.is_err() {
      println!("Failed to send device IO control.");
    } else {
      println!("Sent device IO control! Bytes returned was: {}", res_bytes_returned.unwrap());

      let mut calib = crate::calib::SerializedCalibration::empty_calibration();
      // let mut calib_bytes = [0u8; 156];
      // let mut calib_bytes: [u8; 156] = unsafe { std::mem::transmute(calib) };
      let calib_bytes_ptr = &mut calib as *mut crate::calib::SerializedCalibration;
      let calib_bytes_len = 156;

      let leap_xu_get_calib_req = construct_ksp_node_for_xu_query(
        LEAP_XU_GUID,
        25, // leap_xu.h:132 - LEAP_XU_CALIBRATION_DATA
        Win32CoreAudio::KSPROPERTY_TYPE_GET | Win32CoreAudio::KSPROPERTY_TYPE_TOPOLOGY,
        node_id.0
      );
      let res_bytes_returned = send_device_io_control_kspropxu(
        dev_io_handle,
        leap_xu_get_calib_req,
        calib_bytes_ptr as _,
        calib_bytes_len
      );
      if res_bytes_returned.is_err() {
        println!("Failed to get calibration data.");
      } else {
        // println!("Got calibration data: {:?}", &calib_bytes);
        println!("Calibration bytes as SerializedCalibration struct... {:?}", calib);
      }
    }
  }

  return Ok(());

  // // // let custom_leap_device = naive_wait_and_get_op_results(&WinDevCustom::CustomDevice::FromIdAsync(
  // // //   leap_dev_id,
  // // //   WinDevCustom::DeviceAccessMode::ReadWrite,
  // // //   WinDevCustom::DeviceSharingMode::Shared // or try Exclusive?
  // // // )?, timeout)?;
  // // // println!("Got custom leap device!");

  // // // let mut ext_prop = Win32CoreAudio::KSIDENTIFIER {
  // // //   Anonymous: Win32CoreAudio::KSIDENTIFIER_0 {
  // // //     Anonymous: Win32CoreAudio::KSIDENTIFIER_0_0 {
  // // //       // Set: LEAP_XU_GUID,
  // // //       Set: OMNIVISION_XU_GUID,
  // // //       // Id: Win32CoreAudio::KSPROPERTY_EXTENSION_UNIT_INFO.0 as u32,
  // // //       Id: 0,
  // // //       // Flags: Win32CoreAudio::KSPROPERTY_TYPE_SETSUPPORT | Win32CoreAudio::KSPROPERTY_TYPE_TOPOLOGY,
  // // //       Flags: 0x1000_0100,
  // // //     }
  // // //   }
  // // // };
  
  // // // let mut ext_node = Win32CoreAudio::KSP_NODE {
  // // //   Property: Win32CoreAudio::KSIDENTIFIER {
  // // //     Anonymous: Win32CoreAudio::KSIDENTIFIER_0 {
  // // //       Anonymous: Win32CoreAudio::KSIDENTIFIER_0_0 {
  // // //         Set: LEAP_XU_GUID,
  // // //         // Id: Win32CoreAudio::KSPROPERTY_EXTENSION_UNIT_INFO.0 as u32,
  // // //         Id: 0,
  // // //         // Flags: Win32CoreAudio::KSPROPERTY_TYPE_SETSUPPORT | Win32CoreAudio::KSPROPERTY_TYPE_TOPOLOGY,
  // // //         Flags: 0x1000_0100,
  // // //       }
  // // //     }
  // // //   },
  // // //   NodeId: device_xu_control_nodes[1].0.0,
  // // //   Reserved: 0u32
  // // // };

  // // // // Let's try to send an IOCTL_KS_PROPERTY
  // // // let ksp_node_memsize = std::mem::size_of_val(&ext_node);
  // // // // println!("ksp_node_memsize is {}", ksp_node_memsize); 32
  // // // // // // return Ok(());
  // // // let in_membuf = WinFoundation::MemoryBuffer::Create(ksp_node_memsize as u32)?;
  // // // let in_membuf_ref = in_membuf.CreateReference()?;
  // // // // Write to buffer...
  // // // unsafe {
  // // //   let ksp_node_bytes = std::mem::transmute_copy::<Win32CoreAudio::KSP_NODE, [u8; 32]>(&ext_node);
  // // //   let slice = as_mut_slice(&in_membuf_ref)?;
  // // //   slice.copy_from_slice(&ksp_node_bytes);
  // // // }
  // // // println!("HERE1");
  // // // let req_control_code = WinDevCustom::IOControlCode::CreateIOControlCode(
  // // //   Win32SystemServices::FILE_DEVICE_KS as u16, // ks.h IOCTL_KS_PROPERTY ...
  // // //   0x000,
  // // //   WinDevCustom::IOControlAccessMode::Any,
  // // //   WinDevCustom::IOControlBufferingMethod::Neither,
  // // // );
  // // // if req_control_code.is_err() { println!("Error creating control code: {:?}", req_control_code.as_ref().err()); }
  // // // println!("HERE2");
  // // // let in_buf = WinStorageStreams::Buffer::CreateCopyFromMemoryBuffer(&in_membuf)?;
  // // // let out_buf = WinStorageStreams::Buffer::CreateCopyFromMemoryBuffer(&in_membuf)?;
  // // // let req = &custom_leap_device.SendIOControlAsync(
  // // //   req_control_code.unwrap(),
  // // //   &in_buf,
  // // //   &out_buf,
  // // // );
  // // // let out_membuf = WinStorageStreams::Buffer::CreateMemoryBufferOverIBuffer(out_buf)?;
  // // // let out_membuf_ref = out_membuf.CreateReference()?;
  // // // println!("HERE3");
  // // // if req.is_err() { println!("Couldn't create IOControlAsync request: {:?}", req.as_ref().err()); }
  // // // let ret_val = naive_wait_and_get_op_results(req.as_ref().unwrap(), timeout);
  // // // if ret_val.is_err() {
  // // //   println!("Error with custom_leap_device.SendIOControlAsync: {:?}", ret_val.err());
  // // // } else {
  // // //   println!("Async op returned value {:?}", ret_val.unwrap());
  // // // }
  // // // unsafe {
  // // //   let out_buf_slice = as_mut_slice(&out_membuf_ref)?;
  // // //   println!("out_buf_slice AFTER async: {:?}", out_buf_slice);
  // // // }
  // // // { in_membuf; out_membuf; in_membuf_ref; out_membuf_ref; }
  // // // // // let out_buffer = WinFoundation::MemoryBuffer::Create(ksp_node_memsize as u32)?;
  // // // // // let out_buf_ref = out_buffer.CreateReference()?;
  // // // // // unsafe {
  // // // // //   let out_buf_slice = as_mut_slice(&out_buf_ref)?;
  // // // // //   println!("out_buf_slice BEFORE async: {:?}", out_buf_slice);
  // // // // // }

  // // // // Initialize a MediaCapture object for the Rigel.
  // // // let leap_cap: WinMediaCap::MediaCapture = {

  // // //   let media_cap_init_settings = WinMediaCap::MediaCaptureInitializationSettings::new()?;
    
  // // //   // We're going to set the SourceGroup to the Rigel source group. Constructing the MediaCapture in this way will let us do MediaFrameReader stuff later.
  // // //   let rigel_src_group = WinMediaCap::Frames::MediaFrameSourceGroup::FromIdAsync(leap_dev.Id()?)?;
  // // //   naive_wait_for_async_op(&rigel_src_group, timeout).expect("Failed waiting for MediaFrameSourceGroups");
  // // //   let rigel_src_group: WinMediaCap::Frames::MediaFrameSourceGroup = rigel_src_group.GetResults()?;
  // // //   println!("Got frame source group for Rigel ID: display_name: {}, size: {}", rigel_src_group.DisplayName()?, rigel_src_group.SourceInfos()?.Size()?);

  // // //   media_cap_init_settings.SetSourceGroup(rigel_src_group)?;
  // // //   // media_cap_init_settings.set_video_device_id(rigel_device.id()?)?;

  // // //   // We'd like exclusive control of the device so we can possibly send commands to it to change camera parameters. (Not doing this right now, but maybe later?)
  // // //   media_cap_init_settings.SetSharingMode(WinMediaCap::MediaCaptureSharingMode::ExclusiveControl)?;

  // // //   // We need to access frames from the CPU, so make that preference explicit.
  // // //   media_cap_init_settings.SetMemoryPreference(WinMediaCap::MediaCaptureMemoryPreference::Cpu)?;

  // // //   // The device doesn't provide audio -- and we don't need it anyway.
  // // //   media_cap_init_settings.SetStreamingCaptureMode(WinMediaCap::StreamingCaptureMode::Video)?;

  // // //   // Initialize the MediaCapture object.
  // // //   let media_cap = WinMediaCap::MediaCapture::new()
  // // //     .expect("Failed to create MediaCapture object");
  // // //   let init_req = media_cap.InitializeWithSettingsAsync(media_cap_init_settings)?;
    
  // // //   naive_wait_for_async_act(&init_req, timeout).unwrap();

  // // //   media_cap
  // // // };

  // // // // The catch: The default resolution reported for the Rigel (640x480) won't work. We can pick any other resolution that the device reports as supported, though, and it will work fine.
  // // // //
  // // // // For our purposes here, we'll use 384x384 @ 90fps.

  // // // // Get available MediaEncodingProperties for streaming video from the device.
  // // // let avail_media_stream_props = leap_cap.VideoDeviceController()
  // // //   .expect("Failed to get VideoDeviceController for Rigel capture")
  // // //   .GetAvailableMediaStreamProperties(WinMediaCap::MediaStreamType::VideoRecord)
  // // //   .expect("Failed to get available MediaStreamProperties for VideoRecord.");

  // // // // Find the 384x384 VideoEncodingProperties we're looking for.
  // // // let vep_384x384_90fps = avail_media_stream_props.into_iter()
  // // //   // Only retrieve VideoEncodingProperties, and cast to that type.
  // // //   .filter_map(|mep| {
  // // //     if mep.Type().unwrap().to_string() == "Video" {
  // // //       let vep: WinMediaProps::VideoEncodingProperties = mep.cast().unwrap();
  // // //       println!("Supported VideoEncodingProperties: Width: {}, Height: {}, Framerate: {}", vep.Width().unwrap(), vep.Height().unwrap(), {
  // // //         let frame_rate = vep.FrameRate().unwrap();
  // // //         frame_rate.Numerator().unwrap() / frame_rate.Denominator().unwrap()
  // // //       });
  // // //       Some(vep)
  // // //     } else { None }
  // // //   })
  // // //   // Specifically find the 384x384 encoding properties.
  // // //   .find(|vep|
  // // //     vep.Width().unwrap() == 384 &&
  // // //     vep.Height().unwrap() == 384 &&
  // // //     media_ratio_to_value(&vep.FrameRate().unwrap()) == 90
  // // //   )
  // // //   .expect("Failed to find supported encoding for 384x384 @ 90fps");

  // // // // Set the Rigel to 384x384 @ 90fps.
  // // // naive_wait_for_async_act(
  // // //   &leap_cap.VideoDeviceController()?.SetMediaStreamPropertiesAsync(WinMediaCap::MediaStreamType::VideoRecord, vep_384x384_90fps)
  // // //     .expect("Failed to set Rigel stream to 384x384"),
  // // //   timeout
  // // // ).expect("Failed waiting while setting Rigel stream to 384x384");

  // // // let leap_vdc = leap_cap.VideoDeviceController()?;


  Ok(())
}

/// Constructs a Windows kernel-streaming property node for querying through DeviceIoControl requests about a particular extension unit. You must have the GUID of the extension unit you're interested in and you need to know the node ID for the IKsControl in the device filter's topology.
fn construct_ksp_node_for_xu_query(
  xu_guid: windows::Guid,
  xu_selector_kspropid: u8,
  xu_type_kspropflags: u32,
  device_node_id: u32,
) -> Win32CoreAudio::KSP_NODE {
  Win32CoreAudio::KSP_NODE {
    Property: Win32CoreAudio::KSIDENTIFIER {
      Anonymous: Win32CoreAudio::KSIDENTIFIER_0 {
        Anonymous: Win32CoreAudio::KSIDENTIFIER_0_0 {
          Set: xu_guid,
          Id: xu_selector_kspropid as u32,
          Flags: xu_type_kspropflags,
        }
      }
    },
    NodeId: device_node_id,
    Reserved: 0u32
  }
}

// let out_prop_ptr: *mut T = unsafe { out_prop_val as *mut T };
// let out_prop_len: u32 = std::mem::size_of_val(out_prop_val) as u32;
// let out_prop_ptr: *mut c_void = unsafe { std::mem::transmute(out_prop_ptr) }; 
/// Returns the number of bytes returned into the out_prop. (Come to think of it, this should probably just match out_prop_len...)
fn send_device_io_control_kspropxu(
  device_io_handle: Win32Foundation::HANDLE,
  xu_ksp_node_request: Win32CoreAudio::KSP_NODE,
  out_prop_ptr: *mut c_void,
  out_prop_len: u32
) -> Result<u32> {
  let xu_ksp_node = xu_ksp_node_request;
  let xu_ksp_node_size = std::mem::size_of_val(&xu_ksp_node) as u32;

  // TODO: This probably needs to be created at the start and then held-onto fully outside of this function.
  let mut prop_overlapped: Win32SystemServices::OVERLAPPED = unsafe { std::mem::zeroed() };
  prop_overlapped.hEvent = unsafe { Win32SysThreading::CreateEventA(ptr::null_mut(), false, false, None) };
  let prop_overlapped_ptr = &mut prop_overlapped as *mut _;

  let xu_ksp_node_ptr = &xu_ksp_node as *const Win32CoreAudio::KSP_NODE;
  let mut bytes_returned: Option<u32> = Some(0);
  let ret_val = unsafe { Win32SystemServices::DeviceIoControl(
    device_io_handle,
    Win32CoreAudio::IOCTL_KS_PROPERTY,
    std::mem::transmute(xu_ksp_node_ptr),
    xu_ksp_node_size,
    out_prop_ptr, // lpoutbuffer -> nullptr to get info
    out_prop_len, // output size is zero, we're just getting info
    bytes_returned.as_mut().unwrap(),
    prop_overlapped_ptr
  ) };
  if !ret_val.as_bool() {
    println!("Failed to send DeviceIOControl. Error code: {:?}", unsafe { Win32Debug::GetLastError() });
    print_last_error();
    return Err(windows::Error::new(unsafe { HRESULT::from_win32(Win32Debug::GetLastError().0) }, "Failed to send DeviceIOControl."));
  } else {
    println!("Sent DeviceIOControl!!!!!!");
    println!("Bytes returned was: {:?}", bytes_returned);
  }

  Ok(bytes_returned.unwrap())
}



    // // // // Pass the size as the first multiple of 8 (?) that is at least ks_ident_size.
    // // // // I don't know if that's what the deal is. Maybe Windows just expects at least 32 bytes. 24 doesn't work but values 32 or larger do work.
    // // // let mut pass_size = 0u32;
    // // // loop { pass_size += 8u32; if pass_size > xu_ksp_node_size { break; }}

    // // // // Try querying for support.
    // // // let mut xu_ksp_node = Win32CoreAudio::KSP_NODE {
    // // //   Property: Win32CoreAudio::KSIDENTIFIER {
    // // //     Anonymous: Win32CoreAudio::KSIDENTIFIER_0 {
    // // //       Anonymous: Win32CoreAudio::KSIDENTIFIER_0_0 {
    // // //         Set: query_for_guid,
    // // //         Id: Win32CoreAudio::KSPROPERTY_EXTENSION_UNIT_INFO.0 as _,
    // // //         Flags: Win32CoreAudio::KSPROPERTY_TYPE_GET | Win32CoreAudio::KSPROPERTY_TYPE_TOPOLOGY,
    // // //       }
    // // //     }
    // // //   },
    // // //   NodeId: node_id.0,
    // // //   Reserved: 0u32
    // // // };
    // // // let mut bytes_returned: Option<u32> = Some(0);
    // // // unsafe {
    // // //   // let mut some_data = [0u8; 256];
    // // //   // let some_data_ptr = some_data.as_mut_ptr();

    // // //   let ret = kscontrolnode.KsProperty(
    // // //     &mut ext_prop,
    // // //     pass_size,
    // // //     0 as _,
    // // //     0,
    // // //     bytes_returned.as_mut().unwrap()
    // // //   );
    // // //   if ret.is_err() {
    // // //     println!("KsProperty request returned error: {:?}", ret.err());
    // // //   }
    // // //   println!("bytes_returned is... {}", bytes_returned.as_ref().unwrap())
    // // // };
    // // // println!("Property has set support query returned {} byte response", bytes_returned.unwrap());



/// Returns whether the argument string contains "VID_" followed by either the more recent or older known Leap Motion/Ultraleap USB vendor IDs. Pass any sort of string identifier that includes a VID into this method to quickly get whether the associated device is a Leap Motion or Ultraleap device.
fn contains_leap_vendor_id<'a, S>(str: &'a S) -> bool where String: From<&'a S> {
  let str = String::from(str).to_uppercase();

  str.contains(&crate::usb_info::LEAP_USB_VID_STR_WITH_PREFIX) || str.contains(&crate::usb_info::LEAP_USB_VID_OLD_STR_WITH_PREFIX)
}

fn is_leap_device(dev_info: &WinDevEnum::DeviceInformation) -> windows::Result<bool> {
  let id = dev_info.Id()?.to_string().to_uppercase();

  Ok(id.contains(&crate::usb_info::LEAP_USB_VID_STR.to_uppercase()) || id.contains(&crate::usb_info::LEAP_USB_VID_OLD_STR.to_uppercase()))
}

fn media_ratio_to_value(media_ratio: &WinMediaProps::MediaRatio) -> u32 {
    return media_ratio.Numerator().unwrap() / media_ratio.Denominator().unwrap();
}

fn get_device_prop_hardware_id(dev_info: &WinDevEnum::DeviceInformation) -> BoxResult<String> {
  let props = dev_info.Properties()?;
  let props_iter = props.First().expect("Iter fail");
  loop {
      let iter_kvp = props_iter.Current().expect("Current fail");
      let iter_key_res = iter_kvp.Key();
      if iter_key_res.is_ok() {
        // // println!("\t\t Key: {}", iter_key_res.as_ref().unwrap());
      } else {
        continue;
      }
      let device_hardware_id_str = "System.Devices.DeviceInstanceId".to_uppercase();
      let iter_key_str_upper = iter_key_res.unwrap().to_string_lossy().to_uppercase();

      // Check the String value (if there is one! Not all of them exist or are strings) and return it if the Key matches that for DeviceInstanceId.
      let iter_value_res = iter_kvp.Value();
      if iter_value_res.is_ok() {
          let value = iter_value_res.unwrap();
          // // println!("\t\t Value: {:?}", value);
          let val_str_maybe = HSTRING::try_from(&value);
          if val_str_maybe.is_ok() {
            let val_str = val_str_maybe.unwrap();
              // // println!("\t\t Value str: {:?}", &val_str);

              if iter_key_str_upper.contains(&device_hardware_id_str) {
                return Ok(val_str.to_string());
              }
          }
          // // let bool_maybe = bool::try_from(&value);
          // // if bool_maybe.is_ok() {
          // //     println!("\t\t Value bool: {:?}", bool_maybe.unwrap());
          // // }
      }

      let has_next = props_iter.MoveNext().expect("MoveNext fail");
      if !has_next { break; }
  }
  TestError::err("System.Devices.DeviceInstanceId property not found.")
}

fn print_last_error() {
  println!("Last error: {}", windows::HRESULT::from_thread().message());
}

// Helper Methods - General WinRT //

fn naive_wait_and_get_op_results<T: ::windows::RuntimeType>(async_op: &WinFoundation::IAsyncOperation<T>, loop_limit: u64) -> ::windows::Result<T> {
    naive_wait_for_async_op(async_op, loop_limit).unwrap();
    async_op.GetResults()
}

/// Spin in place waiting for the async operation to complete. Not very graceful.
///
/// TODO: Use a time-based timeout and not a CPU-clock-based timeout.
// fn naive_wait_for_device<T>(device_req: &IAsyncOperation<T>, loop_limit: u64) -> Result<(), &'static str> {
fn naive_wait_for_async_op<T: windows::RuntimeType>(async_op: &WinFoundation::IAsyncOperation<T>, loop_limit: u64) -> std::result::Result<(), &'static str> {
    let mut loops = 0u64;
    loop {
        if async_op.Status().unwrap() == WinFoundation::AsyncStatus::Completed {
            break;
        }

        loops += 1; if loops >= loop_limit { break; }
    }
    if loops >= loop_limit { return Err("Timeout while waiting for the IAsyncOperation to complete."); }

    Ok(())
}

/// Spin in place waiting for the async action to complete. Not very graceful.
///
/// TODO: Use a time-based timeout and not a CPU-clock-based timeout.
// fn naive_wait_for_device<T>(device_req: &IAsyncOperation<T>, loop_limit: u64) -> Result<(), &'static str> {
fn naive_wait_for_async_act(req: &WinFoundation::IAsyncAction, loop_limit: u64) -> std::result::Result<(), &'static str> {
    let mut loops = 0u64;
    loop {
        if req.Status().unwrap() == WinFoundation::AsyncStatus::Completed {
            break;
        }

        loops += 1; if loops >= loop_limit { break; }
    }
    if loops >= loop_limit { return Err("Timeout while waiting for the IAsyncAction to complete.")}

    Ok(())
}
