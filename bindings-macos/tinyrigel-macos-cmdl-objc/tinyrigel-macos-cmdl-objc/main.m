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
    
    AVCaptureVideoDataOutput *captureOutput = [[AVCaptureVideoDataOutput alloc] init];
    __auto_type const captureDispatchQueue = dispatch_queue_create("captureDispatchQueue", NULL);
    [captureOutput setAlwaysDiscardsLateVideoFrames: YES];
    CustomVideoCallbackClass *customVideoCallbackHandler = [[CustomVideoCallbackClass alloc] init];
    [captureOutput setSampleBufferDelegate:customVideoCallbackHandler queue:captureDispatchQueue];
    
    // Clean up.
    [captureSession removeInput:rigelInput];
    
    NSLog(@"Finished enumerating devices.");
}
@end

@implementation CustomVideoCallbackClass

- (void)captureOutput:(AVCaptureOutput *)output
didOutputSampleBuffer:(CMSampleBufferRef)sampleBuffer
       fromConnection:(AVCaptureConnection *)connection {
    CVImageBufferRef imageBuffer = CMSampleBufferGetImageBuffer(sampleBuffer);
    if (!imageBuffer) {
        return;
    }
    
    CVPixelBufferLockBaseAddress(imageBuffer,0);
    
    size_t bytesPerRow = CVPixelBufferGetBytesPerRow(imageBuffer);
    //size_t width = CVPixelBufferGetWidth(imageBuffer);
    size_t height = CVPixelBufferGetHeight(imageBuffer);
    void *src_buff = CVPixelBufferGetBaseAddress(imageBuffer);
    NSData *data = [NSData dataWithBytes:src_buff length:bytesPerRow * height];
    
    CVPixelBufferUnlockBaseAddress(imageBuffer, 0);
    
    UInt8 b0, b1, b2, b3, b4, b5, b6, b7;
    [data getBytes:&b0 range:NSMakeRange(0, sizeof(UInt8))];
    [data getBytes:&b1 range:NSMakeRange(1, sizeof(UInt8))];
    [data getBytes:&b2 range:NSMakeRange(2, sizeof(UInt8))];
    [data getBytes:&b3 range:NSMakeRange(3, sizeof(UInt8))];
    [data getBytes:&b4 range:NSMakeRange(4, sizeof(UInt8))];
    [data getBytes:&b5 range:NSMakeRange(5, sizeof(UInt8))];
    [data getBytes:&b6 range:NSMakeRange(6, sizeof(UInt8))];
    [data getBytes:&b7 range:NSMakeRange(7, sizeof(UInt8))];
    NSLog(@"First 8 bytes of the image: %d %d %d %d %d %d %d %d",
        b0, b1, b2, b3, b4, b5, b6, b7
    );
    
//    NSLog(@"%d", );
}

@end

//
//foo
////
//////
//////  main.swift
//////  tinyrigel-macos-cmdl
//////
//////  Created by Nicholas Benson on 1/17/21.
//////  Copyright © 2021 Nick Benson. All rights reserved.
//////
////
////import Foundation
////import AVFoundation
////
////// The Ultraleap SIR 170 (or "Rigel") reports a model ID containing "VendorID_10550" and "ProductID_4610".
////// USB Vendor ID 10550 corresponds to "LEAP Motion" (or Leap Motion, now Ultraleap).
////// To find the Rigel, we look at enumerated devices and find the first device whose modelID contains both "VendorID_10550" and "ProductID_4610". We could theoretically also check the device's localizedName, which reports as "Rigel," but to keep the methodology straight-forward, we'll just stick with scanning modelIDs.
////
////func printRigelDetails(rigel: AVCaptureDevice) {
////    print(rigel.localizedName)
////    print("\t- has media type of Video? " + rigel.hasMediaType(AVMediaType.video).description)
////    print("\t- is video MediaType: \(rigel.hasMediaType(AVMediaType.video))")
////    print("\t- manufacturer: \(rigel.manufacturer)")
////    print("\t- model ID: \(rigel.modelID)")
////    print("\t- formats: \(rigel.formats)")
////    print("\t- description: \(rigel.description)")
////    print("\t- debugDescription: \(rigel.debugDescription)")
////}
////
////func tryCaptureFrame(rigel: AVCaptureDevice) {rigel
////    var format_384x384_90fps: AVCaptureDevice.Format? = nil;
////    for format in rigel.formats {
////        print(format.formatDescription.presentationDimensions())
////    }
////
//////    rigel.activeFormat
////
////    let captureSession = AVCaptureSession();
////    captureSession.beginConfiguration();
////
////    guard
////        let deviceInput = try? AVCaptureDeviceInput(device: rigel),
////        captureSession.canAddInput(deviceInput)
////        else { return }
////    captureSession.addInput(deviceInput)
////
////    let videoOutput = AVCaptureVideoDataOutput()
////    guard captureSession.canAddOutput(videoOutput) else { return }
////
////
//////    var input = AVCaptureDeviceInput.init(device: rigel);
////
//////    input.
////}
////
////var rigel: AVCaptureDevice? = nil;
////for device in AVCaptureDevice.devices() {
////    let modelID = device.modelID;
////    if modelID.contains("VendorID_10550") && modelID.contains("ProductID_4610") {
////        rigel = device;
////        break;
////    }
////}
////
////if (rigel != nil) {
////    printRigelDetails(rigel: rigel!);
////    tryCaptureFrame(rigel: rigel!);
////}
