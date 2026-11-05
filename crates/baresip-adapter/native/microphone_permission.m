/* Authorization metadata only. Never allocates AVCaptureSession or audio devices. */
#import <AVFoundation/AVFoundation.h>
#import <Foundation/Foundation.h>
#import <dispatch/dispatch.h>

int sip_microphone_permission(int request_access, int *permission) {
    if (!permission) return 1;
    @autoreleasepool {
        @try {
            AVAuthorizationStatus status = [AVCaptureDevice authorizationStatusForMediaType:AVMediaTypeAudio];
            if (request_access && status == AVAuthorizationStatusNotDetermined) {
                /* This function is called from a blocking Rust worker, never the UI thread. */
                dispatch_semaphore_t completed = dispatch_semaphore_create(0);
                [AVCaptureDevice requestAccessForMediaType:AVMediaTypeAudio completionHandler:^(BOOL granted) {
                    (void)granted;
                    dispatch_semaphore_signal(completed);
                }];
                if (dispatch_semaphore_wait(completed, dispatch_time(DISPATCH_TIME_NOW, 180 * NSEC_PER_SEC))) return 1;
                status = [AVCaptureDevice authorizationStatusForMediaType:AVMediaTypeAudio];
            }
            switch (status) {
                case AVAuthorizationStatusNotDetermined: *permission = 0; break;
                case AVAuthorizationStatusRestricted: *permission = 1; break;
                case AVAuthorizationStatusDenied: *permission = 2; break;
                case AVAuthorizationStatusAuthorized: *permission = 3; break;
                default: return 1;
            }
            return 0;
        } @catch (NSException *exception) {
            (void)exception;
            return 1;
        }
    }
}
