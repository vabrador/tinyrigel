//
//  tinyrigel_lib_and_tests.m
//  tinyrigel-lib-and-tests
//

#import "tinyrigel_lib_and_tests.h"

@implementation TinyRigelAVCapture : NSObject {}

- (instancetype)init {
    self = [super init];
    return self;
}

- (void)captureOutput:(AVCaptureOutput *)output
didOutputSampleBuffer:(CMSampleBufferRef)sampleBuffer
       fromConnection:(AVCaptureConnection *)connection {
    
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
    NSLog(@"[Frame Thread] Some bytes: %d %d %d %d %d %d %d %d",
        b0, b1, b2, b3, b4, b5, b6, b7
    );
}

- (void)captureOutput:(AVCaptureOutput *)output
  didDropSampleBuffer:(CMSampleBufferRef)sampleBuffer
       fromConnection:(AVCaptureConnection *)connection {
    
    NSLog(@"Dropped frame");
}

@end

