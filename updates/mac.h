#import <CommonCrypto/CommonDigest.h>
static int runTool(NSString *path,NSArray *args){
 NSTask *t=[NSTask new];t.executableURL=[NSURL fileURLWithPath:path];t.arguments=args;
 if(![t launchAndReturnError:nil])return -1;[t waitUntilExit];return t.terminationStatus;
}
static BOOL validVersion(NSString *v){
 if(![v isKindOfClass:NSString.class])return NO;
 return [v rangeOfString:@"^[0-9]+\\.[0-9]+\\.[0-9]+$" options:NSRegularExpressionSearch].location!=NSNotFound;
}
static void updateFailure(NSString *reason){
 dispatch_async(dispatch_get_main_queue(),^{NSAlert *a=[NSAlert new];a.messageText=@"RMP could not update";a.informativeText=reason;[a runModal];});
}
static NSString *createUpdateStage(NSString *root, NSError **error){
 NSString *path=[root stringByAppendingPathComponent:[@".rmp-update-" stringByAppendingString:NSUUID.UUID.UUIDString]];
 return [NSFileManager.defaultManager createDirectoryAtPath:path withIntermediateDirectories:NO attributes:@{NSFilePosixPermissions:@0700} error:error]?path:nil;
}
@interface RMPUpdateDownload : NSObject <NSURLSessionDownloadDelegate>
@property (nonatomic, strong) NSURLSession *session;
@property (nonatomic, copy) void (^onProgress)(double fraction);
@property (nonatomic, copy) void (^onFinish)(NSURL *file, NSURLResponse *response, NSError *error);
@property (nonatomic) BOOL finished;
@end
@implementation RMPUpdateDownload
- (void)URLSession:(NSURLSession *)s downloadTask:(NSURLSessionDownloadTask *)t didWriteData:(int64_t)written totalBytesWritten:(int64_t)total totalBytesExpectedToWrite:(int64_t)expected {
 if(expected<=0)return;
 double fraction=(double)total/(double)expected;
 dispatch_async(dispatch_get_main_queue(),^{if(self.onProgress)self.onProgress(fraction);});
}
- (void)URLSession:(NSURLSession *)s downloadTask:(NSURLSessionDownloadTask *)t didFinishDownloadingToURL:(NSURL *)location {
 NSString *kept=[NSTemporaryDirectory() stringByAppendingPathComponent:[NSUUID.UUID.UUIDString stringByAppendingString:@".zip"]];
 NSError *moveError=nil;BOOL moved=[NSFileManager.defaultManager moveItemAtPath:location.path toPath:kept error:&moveError];
 self.finished=YES;
 NSURLResponse *response=t.response;
 dispatch_async(dispatch_get_main_queue(),^{if(self.onFinish)self.onFinish(moved?[NSURL fileURLWithPath:kept]:nil,response,moved?nil:moveError);});
 [self.session finishTasksAndInvalidate];
}
- (void)URLSession:(NSURLSession *)s task:(NSURLSessionTask *)t didCompleteWithError:(NSError *)error {
 if(!error||self.finished)return;
 NSURLResponse *response=t.response;
 dispatch_async(dispatch_get_main_queue(),^{if(self.onFinish)self.onFinish(nil,response,error);});
 [self.session finishTasksAndInvalidate];
}
@end

static NSWindow *updateWindow;
static NSTextField *updateLabel;
static NSProgressIndicator *updateBar;
static NSButton *updateCancelButton,*updateRestartButton;
static NSTimer *countdownTimer;
static RMPUpdateDownload *activeDownload;
static NSString *stagedZip;
static void (^installNow)(void);

static void closeUpdateWindow(void){
 [countdownTimer invalidate];countdownTimer=nil;installNow=nil;activeDownload=nil;
 [updateWindow orderOut:nil];
}
static void showUpdateWindow(void){
 if(updateWindow){[updateWindow makeKeyAndOrderFront:nil];return;}
 updateWindow=[[NSWindow alloc]initWithContentRect:NSMakeRect(0,0,440,140) styleMask:NSWindowStyleMaskTitled backing:NSBackingStoreBuffered defer:NO];
 updateWindow.title=@"Updating RMP";updateWindow.releasedWhenClosed=NO;
 NSView *v=updateWindow.contentView;
 updateLabel=[NSTextField labelWithString:@""];updateLabel.frame=NSMakeRect(20,96,400,20);[v addSubview:updateLabel];
 updateBar=[[NSProgressIndicator alloc]initWithFrame:NSMakeRect(20,64,400,20)];
 updateBar.style=NSProgressIndicatorStyleBar;updateBar.indeterminate=NO;updateBar.minValue=0;updateBar.maxValue=1;[v addSubview:updateBar];
 updateCancelButton=[NSButton buttonWithTitle:@"Cancel" target:NSApp action:nil];
 updateCancelButton.frame=NSMakeRect(20,20,100,30);[v addSubview:updateCancelButton];
 updateRestartButton=[NSButton buttonWithTitle:@"Restart Now" target:NSApp action:nil];
 updateRestartButton.frame=NSMakeRect(320,20,100,30);updateRestartButton.hidden=YES;[v addSubview:updateRestartButton];
 [updateWindow center];[updateWindow makeKeyAndOrderFront:nil];
}
/* Button handling is wired in the owning app delegate by the update actions below. */
@interface RMPUpdateActions : NSObject
+ (void)cancelPressed;
+ (void)restartPressed;
@end
@implementation RMPUpdateActions
+ (void)cancelPressed {
 if(activeDownload){
  RMPUpdateDownload *dl=activeDownload;dl.onProgress=nil;dl.onFinish=nil;activeDownload=nil;
  [dl.session invalidateAndCancel];closeUpdateWindow();return;
 }
 if(stagedZip)[NSFileManager.defaultManager removeItemAtPath:stagedZip error:nil];
 stagedZip=nil;closeUpdateWindow();
}
+ (void)restartPressed {
 void (^install)(void)=installNow;installNow=nil;
 [countdownTimer invalidate];countdownTimer=nil;
 if(install)install();
}
@end

static void installDownloadedUpdate(NSString *zipPath,NSString *version);

static void startCountdown(NSString *zipPath,NSString *version){
 __block int remaining=5;
 stagedZip=zipPath;
 updateBar.doubleValue=1;
 updateLabel.stringValue=[NSString stringWithFormat:@"RMP %@ is ready. Restarting in %d seconds to finish the update.",version,remaining];
 updateRestartButton.hidden=NO;
 installNow=^{stagedZip=nil;closeUpdateWindow();installDownloadedUpdate(zipPath,version);};
 countdownTimer=[NSTimer scheduledTimerWithTimeInterval:1 repeats:YES block:^(NSTimer *timer){
  remaining--;
  if(remaining<=0){
   [timer invalidate];countdownTimer=nil;
   void (^install)(void)=installNow;installNow=nil;stagedZip=nil;
   if(install)install();
   return;
  }
  updateLabel.stringValue=[NSString stringWithFormat:@"RMP %@ is ready. Restarting in %d seconds to finish the update.",version,remaining];
 }];
}

static void installDownloadedUpdate(NSString *zipPath,NSString *version){
 NSFileManager *fm=NSFileManager.defaultManager;NSString *dest=NSBundle.mainBundle.bundlePath;
 NSString *root=dest.stringByDeletingLastPathComponent;
 NSError *installError=nil;
 NSString *stage=createUpdateStage(root,&installError);
 if(!stage){
  root=[NSHomeDirectory() stringByAppendingPathComponent:@"Applications"];
  dest=[root stringByAppendingPathComponent:@"RMP.app"];
  if(![fm createDirectoryAtPath:root withIntermediateDirectories:YES attributes:nil error:&installError] ||
     !(stage=createUpdateStage(root,&installError))){
   [fm removeItemAtPath:zipPath error:nil];
   updateFailure([NSString stringWithFormat:@"Could not prepare an update in %@. %@",root,installError.localizedDescription]);return;
  }
 }
 NSString *app=[stage stringByAppendingPathComponent:@"RMP.app"];
 if(runTool(@"/usr/bin/ditto",@[@"-x",@"-k",zipPath,stage])||runTool(@"/usr/bin/codesign",@[@"--verify",@"--deep",@"--strict",app])||![[NSBundle bundleWithPath:app].bundleIdentifier isEqual:NSBundle.mainBundle.bundleIdentifier]||![[[NSBundle bundleWithPath:app] objectForInfoDictionaryKey:@"CFBundleShortVersionString"] isEqual:version]){
 [fm removeItemAtPath:stage error:nil];[fm removeItemAtPath:zipPath error:nil];updateFailure(@"The downloaded app failed verification.");return;}
 [fm removeItemAtPath:zipPath error:nil];
 NSTask *helper=[NSTask new];helper.executableURL=[NSURL fileURLWithPath:@"/bin/bash"];
 helper.arguments=@[[app stringByAppendingPathComponent:@"Contents/Resources/mac-install.sh"],[@(NSProcessInfo.processInfo.processIdentifier) stringValue],stage,dest];
 if(![helper launchAndReturnError:nil]){[fm removeItemAtPath:stage error:nil];updateFailure(@"The update helper could not start.");return;}
 dispatch_async(dispatch_get_main_queue(),^{[NSApp terminate:nil];});
}

static void stageUpdate(NSDictionary *asset,NSString *version){
 NSURL *url=[NSURL URLWithString:asset[@"browser_download_url"]];
 showUpdateWindow();
 updateLabel.stringValue=[NSString stringWithFormat:@"Downloading RMP %@…",version];
 updateBar.doubleValue=0;
 updateCancelButton.target=RMPUpdateActions.class;updateCancelButton.action=@selector(cancelPressed);
 updateRestartButton.target=RMPUpdateActions.class;updateRestartButton.action=@selector(restartPressed);
 RMPUpdateDownload *dl=[RMPUpdateDownload new];activeDownload=dl;
 dl.session=[NSURLSession sessionWithConfiguration:NSURLSessionConfiguration.ephemeralSessionConfiguration delegate:dl delegateQueue:nil];
 dl.onProgress=^(double fraction){
  updateBar.doubleValue=fraction;
  updateLabel.stringValue=[NSString stringWithFormat:@"Downloading RMP %@… %d%%",version,(int)(fraction*100)];
 };
 dl.onFinish=^(NSURL *file,NSURLResponse *response,NSError *error){
  activeDownload=nil;
  if(error || !file || [(NSHTTPURLResponse *)response statusCode]!=200){closeUpdateWindow();updateFailure(@"The download failed. Try again next time you open RMP.");return;}
  NSData *bytes=[NSData dataWithContentsOfURL:file options:NSDataReadingMappedIfSafe error:nil];
  unsigned char hash[CC_SHA256_DIGEST_LENGTH];CC_SHA256(bytes.bytes,(CC_LONG)bytes.length,hash);
  NSMutableString *digest=[@"sha256:" mutableCopy];for(unsigned i=0;i<sizeof(hash);i++)[digest appendFormat:@"%02x",hash[i]];
  if(![digest isEqual:asset[@"digest"]]){[NSFileManager.defaultManager removeItemAtURL:file error:nil];closeUpdateWindow();updateFailure(@"The download did not match the release checksum.");return;}
  startCountdown(file.path,version);
 };
 NSURLSessionDownloadTask *task=[dl.session downloadTaskWithURL:url];
 [task resume];
}

static BOOL checkingUpdates=NO, explicitCheck=NO;
static void finishCheck(NSString *title,NSString *details){
 dispatch_async(dispatch_get_main_queue(),^{
 BOOL show=explicitCheck;explicitCheck=NO;checkingUpdates=NO;
 if(show){NSAlert *a=[NSAlert new];a.messageText=title;a.informativeText=details;[a runModal];}
 });
}
static void checkMacUpdates(BOOL manual){
 NSString *current=[NSBundle.mainBundle objectForInfoDictionaryKey:@"CFBundleShortVersionString"];if(!validVersion(current))return;
 explicitCheck|=manual;if(checkingUpdates)return;checkingUpdates=YES;
 NSMutableURLRequest *req=[NSMutableURLRequest requestWithURL:[NSURL URLWithString:@"https://api.github.com/repos/KMThegamer2022/RMP/releases?per_page=100"]];[req setValue:@"RMP-Updater" forHTTPHeaderField:@"User-Agent"];req.timeoutInterval=15;req.cachePolicy=NSURLRequestReloadIgnoringLocalCacheData;
 [[[NSURLSession sharedSession] dataTaskWithRequest:req completionHandler:^(NSData *data,NSURLResponse *response,NSError *error){
 if(error||[(NSHTTPURLResponse *)response statusCode]!=200){finishCheck(@"Could not check for updates",error.localizedDescription?:[NSString stringWithFormat:@"GitHub returned HTTP %ld.",(long)[(NSHTTPURLResponse *)response statusCode]]);return;}
 id releases=[NSJSONSerialization JSONObjectWithData:data options:0 error:nil];if(![releases isKindOfClass:NSArray.class]){finishCheck(@"Could not check for updates",@"GitHub returned an unexpected response.");return;}
 NSString *best=current,*notes=nil;NSDictionary *asset=nil;
 for(NSDictionary *r in releases){if([r[@"prerelease"] boolValue]||[r[@"draft"] boolValue])continue;
 NSString *tag=r[@"tag_name"];if(![tag hasPrefix:@"v"])continue;
 NSString *v=[tag substringFromIndex:1];if([v hasSuffix:@"-macos"])v=[v substringToIndex:v.length-6];
 if(!validVersion(v)||[v compare:best options:NSNumericSearch]!=NSOrderedDescending)continue;
 for(NSDictionary *a in r[@"assets"]){NSString *name=a[@"name"];
 if(![name hasPrefix:@"RMP-"]||![name hasSuffix:@"macOS-arm64.zip"]||[name containsString:@"/"])continue;
 NSString *expected=[NSString stringWithFormat:@"https://github.com/KMThegamer2022/RMP/releases/download/%@/%@",tag,name];
 if(![expected isEqual:a[@"browser_download_url"]]||![a[@"digest"] isKindOfClass:NSString.class]||![a[@"digest"] hasPrefix:@"sha256:"])continue;
 best=v;asset=a;notes=[r[@"body"] isKindOfClass:NSString.class]?r[@"body"]:@"";break;}
 }
 if(!asset){finishCheck(@"RMP is up to date",[NSString stringWithFormat:@"You are running RMP %@.",current]);return;}
 dispatch_async(dispatch_get_main_queue(),^{NSAlert *a=[NSAlert new];a.messageText=[NSString stringWithFormat:@"Update to RMP %@?",best];a.informativeText=@"RMP will download the update and reopen. If this copy is read-only, the update will be installed in your personal Applications folder.";
 NSScrollView *scroll=[[NSScrollView alloc]initWithFrame:NSMakeRect(0,0,400,160)];scroll.hasVerticalScroller=YES;NSTextView *text=[[NSTextView alloc]initWithFrame:scroll.bounds];text.editable=NO;text.string=notes;scroll.documentView=text;a.accessoryView=scroll;
 [a addButtonWithTitle:@"Update"];[a addButtonWithTitle:@"Not now"];NSModalResponse answer=[a runModal];checkingUpdates=NO;explicitCheck=NO;if(answer==NSAlertFirstButtonReturn)stageUpdate(asset,best);
 });
 }] resume];
}
