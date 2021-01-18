//
//  main.m
//  tinyrigel-macos-cmdl-objc
//

#import <Foundation/Foundation.h>
#import <AVFoundation/AVFoundation.h>

@interface Tests : NSObject {}

+ (void)rigelDeviceTest;

@end

@interface CustomVideoCallbackClass : NSObject <AVCaptureVideoDataOutputSampleBufferDelegate>

- (void)captureOutput:(AVCaptureOutput *)output
didOutputSampleBuffer:(CMSampleBufferRef)sampleBuffer
       fromConnection:(AVCaptureConnection *)connection;

- (void)captureOutput:(AVCaptureOutput *)output
didDropSampleBuffer:(CMSampleBufferRef)sampleBuffer
     fromConnection:(AVCaptureConnection *)connection;

@end

int main(int argc, const char * argv[]) {
    @autoreleasepool {
        [Tests rigelDeviceTest];
    }
    return 0;
}

@implementation Tests

+ (void) rigelDeviceTest {
    NSLog(@"Begin device enumeration...");
    
    // The Ultraleap SIR 170 (or "Rigel") reports a model ID containing "VendorID_10550" and "ProductID_4610".
    //
    // USB Vendor ID 10550 corresponds to "LEAP Motion" (or Leap Motion, now Ultraleap).
    //
    // To find the Rigel, we look at enumerated devices and find the first device whose modelID contains both "VendorID_10550" and "ProductID_4610". We could theoretically also check the device's localizedName, which reports as "Rigel," but to keep the methodology straight-forward, we'll just stick with scanning modelIDs.
    __auto_type const devices = AVCaptureDevice.devices;
    AVCaptureDevice *rigel = nil;
    for (AVCaptureDevice *device in devices) {
        NSString *modelID = device.modelID;
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
    __auto_type const formats = rigel.formats;
    AVCaptureDeviceFormat *format_384x384_90fps = nil;
    for (AVCaptureDeviceFormat *format in formats) {
        NSLog(@"Format: %@", format);
        __auto_type const formatDesc = format.formatDescription;
        
        if (CMFormatDescriptionGetMediaType(formatDesc) == kCMMediaType_Video) {
            __auto_type const dimensions = CMVideoFormatDescriptionGetDimensions(formatDesc);
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

    NSError *error = nil;
    AVCaptureDeviceInput *rigelInput = [AVCaptureDeviceInput deviceInputWithDevice:rigel error:&error];
    if (!rigelInput) {
        NSLog(@"Error getting rigel input. %@", error);
        return;
    }
    NSLog(@"Initialized rigel input.");
    
    // Set up capture session and output.
    AVCaptureSession *captureSession = [[AVCaptureSession alloc] init];
    if (![captureSession canAddInput:rigelInput]) {
        NSLog(@"Failed to add rigelInput to a new AVCaptureSession.");
        return;
    }
    [captureSession addInput:rigelInput];
    
//    __auto_type preset = [captureSession sessionPreset];
    
    AVCaptureVideoDataOutput *captureOutput = [[AVCaptureVideoDataOutput alloc] init];
    __auto_type const captureDispatchQueue = dispatch_queue_create("captureDispatchQueue", NULL);
    [captureOutput setAlwaysDiscardsLateVideoFrames: YES];
    CustomVideoCallbackClass *customVideoCallbackHandler = [[CustomVideoCallbackClass alloc] init];
    [captureOutput setSampleBufferDelegate:customVideoCallbackHandler queue:captureDispatchQueue];
    
    __auto_type const videoSettings = captureOutput.videoSettings;
    NSLog(@"Video settings: %@", videoSettings);
    __auto_type const availCodecTypes = captureOutput.availableVideoCodecTypes;
    NSLog(@"availCodecTypes: %@", availCodecTypes);
    __auto_type const availPixelFormatTypes = captureOutput.availableVideoCVPixelFormatTypes;
    NSLog(@"availPixelFormatTypes: %@", availPixelFormatTypes);
    __auto_type const mostEfficientPixelFormatType = captureOutput.availableVideoCVPixelFormatTypes[0];
    NSLog(@"mostEfficientPixelFormatType: %@", mostEfficientPixelFormatType);
    NSLog(@"kCVPixelFormatType_422YpCbCr8: %u", kCVPixelFormatType_422YpCbCr8);
    NSLog(@"kCVPixelFormatType_422YpCbCr8_yuvs: %u", kCVPixelFormatType_422YpCbCr8_yuvs);
    
    [captureOutput setVideoSettings:[NSDictionary dictionaryWithObject: [NSNumber numberWithInt:kCVPixelFormatType_422YpCbCr8] forKey:(id)kCVPixelBufferPixelFormatTypeKey]];
    NSLog(@"Set captureOutput videoSettings pixel format.");
    
    if ([captureSession canAddOutput:captureOutput]) {
        [captureSession addOutput:captureOutput];
    } else {
        NSLog(@"Failed to add output to captureSession...");
    }
    NSLog(@"Added output to captureSession.");
    
    // Start...
    NSLog(@"Calling captureSession startRunning.");
    [captureSession startRunning];
    
    NSLog(@"Capture session isRunning? %hhd", [captureSession isRunning]);
    
    // TODO: Only the first frame callback is containing nonzero bytes. What's going on?
    // Could possibly be:
    //   - Misconfiguration of the Rigel -- say, the 640x480 "bad" config?
    //   - Bad code in the frame callback -- some memory nonsense?
    
    int million = 1000000;
    for (int i = 0; i < 500 * million; i++) {}
    // Spin...
//    for (int i = 0; i < 10; i++) {
//        NSLog(@"Spinning... %d", i);
//    }
    
    // Stop...
    [captureSession stopRunning];
    
    // Clean up.
    [captureSession removeInput:rigelInput];
    [captureSession removeOutput:captureOutput];
    
    NSLog(@"Finished enumerating devices.");
}
@end

@implementation CustomVideoCallbackClass

- (void)captureOutput:(AVCaptureOutput *)output
didOutputSampleBuffer:(CMSampleBufferRef)sampleBuffer
       fromConnection:(AVCaptureConnection *)connection {
    NSLog(@"HI FROM CAM THREAD");
    
    CVImageBufferRef imageBuffer = CMSampleBufferGetImageBuffer(sampleBuffer);
    if (!imageBuffer) {
        return;
    }
    
    CVPixelBufferLockBaseAddress(imageBuffer, 0);
    
    size_t bytesPerRow = CVPixelBufferGetBytesPerRow(imageBuffer);
    //size_t width = CVPixelBufferGetWidth(imageBuffer);
    size_t height = CVPixelBufferGetHeight(imageBuffer);
    void *src_buff = CVPixelBufferGetBaseAddress(imageBuffer);
    NSData *data = [NSData dataWithBytes:src_buff length:bytesPerRow * height];
    
    CVPixelBufferUnlockBaseAddress(imageBuffer, 0);
    
    const size_t row = bytesPerRow;
    const size_t halfRow = bytesPerRow * 0.5;
    UInt8 b0, b1, b2, b3, b4, b5, b6, b7;
    [data getBytes:&b0 range:NSMakeRange(00 * row + halfRow, sizeof(UInt8))];
    [data getBytes:&b1 range:NSMakeRange(10 * row + halfRow, sizeof(UInt8))];
    [data getBytes:&b2 range:NSMakeRange(20 * row + halfRow, sizeof(UInt8))];
    [data getBytes:&b3 range:NSMakeRange(30 * row + halfRow, sizeof(UInt8))];
    [data getBytes:&b4 range:NSMakeRange(40 * row + halfRow, sizeof(UInt8))];
    [data getBytes:&b5 range:NSMakeRange(50 * row + halfRow, sizeof(UInt8))];
    [data getBytes:&b6 range:NSMakeRange(60 * row + halfRow, sizeof(UInt8))];
    [data getBytes:&b7 range:NSMakeRange(70 * row + halfRow, sizeof(UInt8))];
    NSLog(@"First 8 bytes of the image: %d %d %d %d %d %d %d %d",
        b0, b1, b2, b3, b4, b5, b6, b7
    );
    
    NSLog(@"End of cam callback");
}

- (void)captureOutput:(AVCaptureOutput *)output
didDropSampleBuffer:(CMSampleBufferRef)sampleBuffer
       fromConnection:(AVCaptureConnection *)connection {
    NSLog(@"Dropped frame");
}

@end
