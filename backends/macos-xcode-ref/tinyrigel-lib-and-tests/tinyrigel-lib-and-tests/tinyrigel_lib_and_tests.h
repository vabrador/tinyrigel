//
//  tinyrigel_lib_and_tests.h
//  tinyrigel-lib-and-tests
//

#import <Foundation/Foundation.h>
#import <AVFoundation/AVFoundation.h>

@interface TinyRigelAVCapture : NSObject <AVCaptureVideoDataOutputSampleBufferDelegate>

- (instancetype)init;

- (void)captureOutput:(AVCaptureOutput *)output
didOutputSampleBuffer:(CMSampleBufferRef)sampleBuffer
       fromConnection:(AVCaptureConnection *)connection;

- (void)captureOutput:(AVCaptureOutput *)output
didDropSampleBuffer:(CMSampleBufferRef)sampleBuffer
     fromConnection:(AVCaptureConnection *)connection;

@end
