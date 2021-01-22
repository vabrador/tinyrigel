//
//  Tests.m
//  Tests
//

#import <XCTest/XCTest.h>
#import "tinyrigel_lib_and_tests.h"

@interface Tests : XCTestCase

@end

@implementation Tests

- (void)setUp {
    // Put setup code here. This method is called before the invocation of each test method in the class.
}

- (void)tearDown {
    // Put teardown code here. This method is called after the invocation of each test method in the class.
}

- (void)testRigelFrame {
    NSLog(@"Begin device enumeration...");
    
    // The Ultraleap SIR 170 (or "Rigel") reports a model ID containing "VendorID_10550" and "ProductID_4610".
    //
    // USB Vendor ID 10550 corresponds to "LEAP Motion" (or Leap Motion, now Ultraleap).
    //
    // To find the Rigel, we look at enumerated devices and find the first device whose modelID contains both "VendorID_10550" and "ProductID_4610". We could theoretically also check the device's localizedName, which reports as "Rigel," but to keep the methodology straight-forward, we'll just stick with scanning modelIDs.
    NSArray<AVCaptureDevice *> * const devices = [AVCaptureDevice devices];
    AVCaptureDevice * rigel = nil;
    for (int i = 0; i < [devices count]; i++) {
        AVCaptureDevice * device = [devices objectAtIndex: i];
        
        NSString * modelID = [device modelID];
        if ([modelID rangeOfString:@"VendorID_10550"].location == NSNotFound) {
            continue;
        }
        if ([modelID rangeOfString:@"ProductID_4610"].location == NSNotFound) {
            continue;
        }
        rigel = device;
        break;
    }
    if (rigel == nil) {
        NSLog(@"No Rigel found in enumerated devices. Is your Rigel plugged in?");
        return;
    }
    NSLog(@"Found connected Rigel: %@", rigel);
        
    // Once we have a Rigel, we need to find the correct format to use.
    //
    // On Windows but possibly not on other systems, the default format reported by the Rigel is not actually valid, and will prevent the Rigel from capturing frames.
    //
    // Regardless, in this case, we're interested in 384x384 capture @ 90 fps.
    NSArray<AVCaptureDeviceFormat *> * const formats = [rigel formats];
    AVCaptureDeviceFormat *format_384x384_90fps = nil;
    for (AVCaptureDeviceFormat *format in formats) {
        NSLog(@"Format: %@", format);
        CMFormatDescriptionRef const formatDesc = [format formatDescription];
        
        if (CMFormatDescriptionGetMediaType(formatDesc) == kCMMediaType_Video) {
            CMVideoDimensions const dimensions = CMVideoFormatDescriptionGetDimensions(formatDesc);
            NSLog(@"Dimensions: %dx%d", dimensions.width, dimensions.height);
            if (dimensions.width == 384 && dimensions.height == 384) {
                format_384x384_90fps = format;
                break;
            }
        }
    }
    if (format_384x384_90fps == nil) {
        NSLog(@"No 384x384 format found in Rigel's reported resolutions. Aborting.");
        return;
    }
    NSLog(@"Successfully found 384x384 @ 90fps format.");
    
    // Lock the Rigel configuration and set its format. We'll unlock after invoking startRunning, per Apple docs.
    // https://developer.apple.com/documentation/avfoundation/avcapturedevice/1387810-lockforconfiguration
    NSError *configError;
    BOOL hadConfigError = ![rigel lockForConfiguration: &configError];
    if (hadConfigError) {
        NSLog(@"Error locking Rigel for configuration: %@", configError);
        return;
    }
    
    // This gives us time to test the lockForConfiguration failure code on the Rust side, by running this, then running the Rust variant at the same time while the Rigel is locked.
    // TODO: Probably don't need this after the code is validated, so can remove.
    NSLog(@"LOCKED RIGEL");
    for (int i = 0; i < 1000 * 1000000; i++) {}
    
    // Set the Rigel to the format we want.
    [rigel setActiveFormat: format_384x384_90fps];
    NSLog(@"Set Rigel to 384x384 @ 90fps format.");

    NSError *error = nil;
    AVCaptureDeviceInput *rigelInput = [AVCaptureDeviceInput deviceInputWithDevice: rigel error: &error];
    if (!rigelInput) {
        NSLog(@"Error getting rigel input. %@", error);
        return;
    }
    NSLog(@"Initialized rigel input.");
    
    // Set up capture session.
    AVCaptureSession *captureSession = [AVCaptureSession alloc];
    captureSession = [captureSession init];
    if (![captureSession canAddInput:rigelInput]) {
        NSLog(@"Unable to add rigelInput to a new AVCaptureSession.");
        return;
    }
    [captureSession addInput:rigelInput];
    
    // Set up capture output connection.
    AVCaptureVideoDataOutput *captureOutput = [AVCaptureVideoDataOutput alloc];
    captureOutput = [captureOutput init];
    dispatch_queue_t captureDispatchQueue = dispatch_queue_create(NULL, DISPATCH_QUEUE_SERIAL);
    [captureOutput setAlwaysDiscardsLateVideoFrames: YES];
    TinyRigelAVCapture *tinyRigelAVCap = [[TinyRigelAVCapture alloc] init];
    [captureOutput setSampleBufferDelegate:tinyRigelAVCap queue:captureDispatchQueue];
    
    // Print some information about the capture output configuration.
    __auto_type const videoSettings = captureOutput.videoSettings;
    NSLog(@"Video settings: %@", videoSettings);
    __auto_type const availPixelFormatTypes = captureOutput.availableVideoCVPixelFormatTypes;
    NSLog(@"availPixelFormatTypes: %@", availPixelFormatTypes);
    __auto_type const mostEfficientPixelFormatType = captureOutput.availableVideoCVPixelFormatTypes[0];
    NSLog(@"mostEfficientPixelFormatType: %@", mostEfficientPixelFormatType);
    NSLog(@"kCVPixelFormatType_422YpCbCr8: %u", kCVPixelFormatType_422YpCbCr8);
    NSLog(@"kCVPixelFormatType_422YpCbCr8_yuvs: %u", kCVPixelFormatType_422YpCbCr8_yuvs);
    
    // Set the capture output configuration to '2vuy', AKA YUY2, which is the format that the Rigel claims to output. (Spoiler: It actually does not output YUY2 data.)
    
    [captureOutput setVideoSettings:[NSDictionary dictionaryWithObject: [NSNumber numberWithInt:kCVPixelFormatType_422YpCbCr8] forKey:(id)kCVPixelBufferPixelFormatTypeKey]];
    NSLog(@"Set captureOutput videoSettings pixel format to %u, aka '2vuy', via kCVPixelFormatType_422YpCbCr8.", kCVPixelFormatType_422YpCbCr8);
    
    // Attempt to add the configured output node to the capture session.
    if ([captureSession canAddOutput:captureOutput]) {
        [captureSession addOutput:captureOutput];
    } else {
        NSLog(@"Failed to add output to captureSession...");
    }
    NSLog(@"Added output to captureSession.");
    
    // Start the capture session.
    NSLog(@"Calling captureSession startRunning.");
    [captureSession startRunning];
    
    // Once the capture session has started, we don't need the Rigel's configuration locked any more. (Is this really true? Or should we keep it locked for the duration of the capture session?)
    [rigel unlockForConfiguration];
    NSLog(@"Unlocked Rigel configuration now that startRunning has been invoked.");
    NSLog(@"Capture session isRunning? %hhd", [captureSession isRunning]);
    
    // Let the capture session run for a little while. We'll get callbacks from the capture thread.
    for (int i = 0; i < 500 * 1000000; i++) {}
    
    // Stop...
    [captureSession stopRunning];
    
    // Clean up.
    [captureSession removeInput:rigelInput];
    [captureSession removeOutput:captureOutput];
    NSLog(@"Finished enumerating devices.");
    
    XCTAssert(YES);
}

- (void)testPerformanceExample {
    // This is an example of a performance test case.
    [self measureBlock:^{
        // Put the code you want to measure the time of here.
    }];
}

@end
