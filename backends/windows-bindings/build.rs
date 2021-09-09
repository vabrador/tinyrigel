fn main() {
  windows::build! {
    Windows::Devices::*,
    Windows::Devices::Enumeration::*,
    Windows::Devices::Custom::*,
    Windows::Devices::Usb::*,
    Windows::Foundation::*,
    Windows::Foundation::Collections::*,
    Windows::Media::Capture::*,
    Windows::Media::Capture::Frames::*,
    Windows::Media::Devices::*,
    Windows::Media::MediaProperties::*,
    Windows::Storage::Streams::*,
    Windows::Win32::System::WinRT::IMemoryBufferByteAccess,

    // // Windows Devices USB test...
    // Windows::Devices::Usb::*,

    // // Device Capture API test...
    // Windows::Graphics::*,
    // Windows::Graphics::Imaging::*,
    // Windows::Media::*,
    // Windows::Media::Capture::*,
    // Windows::Media::Capture::MediaCapture,
    // Windows::Media::Capture::Frames::*,
    // Windows::Media::Devices::*,
    // Windows::Media::MediaProperties::*,
    // Windows::Storage::Streams::*,

    // WinUSB test...
    Windows::Win32::Devices::Usb::*,
    Windows::Win32::Devices::DeviceAndDriverInstallation::*,
    Windows::Win32::Foundation::*,
    Windows::Win32::System::Diagnostics::Debug::*,
    Windows::Win32::Storage::FileSystem::*,

    // DirectShow test...
    Windows::Win32::System::Com::CoCreateInstance,
    Windows::Win32::System::Com::CoInitialize,
    Windows::Win32::System::Com::CoInitializeEx,
    Windows::Win32::System::Com::IEnumMoniker,
    Windows::Win32::System::Com::IMoniker,
    Windows::Win32::System::Com::IBindCtx,
    Windows::Win32::System::Com::CoGetMalloc,
    Windows::Win32::System::Com::IMalloc,
    Windows::Win32::System::Com::IPropertyBag2,
    Windows::Win32::System::Com::IDataObject,
    Windows::Win32::System::Com::*,
    Windows::Win32::System::OleAutomation::IPropertyBag,
    Windows::Win32::System::OleAutomation::VariantInit,
    Windows::Win32::System::OleAutomation::VARIANT,
    Windows::Win32::System::OleAutomation::VariantClear,
    Windows::Win32::System::OleAutomation::*,
    // Windows::Win32::System::Com::*,
    Windows::Win32::Graphics::DirectShow::*,
    Windows::Win32::Media::Audio::CoreAudio::*,
    Windows::Win32::System::SystemServices::*,
    Windows::Win32::System::Threading::*,

    // Windows::Win32::System::Com::CoInitializeSecurity
  };
}
