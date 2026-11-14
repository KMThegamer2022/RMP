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
static void stageUpdate(NSDictionary *asset,NSString *version){
 NSURL *url=[NSURL URLWithString:asset[@"browser_download_url"]];
 [[[NSURLSession sharedSession] downloadTaskWithURL:url completionHandler:^(NSURL *file,NSURLResponse *response,NSError *error){
 if(error || !file || [(NSHTTPURLResponse *)response statusCode]!=200){updateFailure(@"The download failed. Try again next time you open RMP.");return;}
 NSData *bytes=[NSData dataWithContentsOfURL:file options:NSDataReadingMappedIfSafe error:nil];
 unsigned char hash[CC_SHA256_DIGEST_LENGTH];CC_SHA256(bytes.bytes,(CC_LONG)bytes.length,hash);
 NSMutableString *digest=[@"sha256:" mutableCopy];for(unsigned i=0;i<sizeof(hash);i++)[digest appendFormat:@"%02x",hash[i]];
 if(![digest isEqual:asset[@"digest"]]){updateFailure(@"The download did not match the release checksum.");return;}
 NSFileManager *fm=NSFileManager.defaultManager;NSString *dest=NSBundle.mainBundle.bundlePath;
 NSString *stage=[dest.stringByDeletingLastPathComponent stringByAppendingPathComponent:[@".rmp-update-" stringByAppendingString:NSUUID.UUID.UUIDString]];
 if(![fm createDirectoryAtPath:stage withIntermediateDirectories:NO attributes:@{NSFilePosixPermissions:@0700} error:nil]){updateFailure(@"Move RMP into a writable Applications folder, then reopen it.");return;}
 NSString *app=[stage stringByAppendingPathComponent:@"RMP.app"];
 if(runTool(@"/usr/bin/ditto",@[@"-x",@"-k",file.path,stage])||runTool(@"/usr/bin/codesign",@[@"--verify",@"--deep",@"--strict",app])||![[NSBundle bundleWithPath:app].bundleIdentifier isEqual:NSBundle.mainBundle.bundleIdentifier]||![[[NSBundle bundleWithPath:app] objectForInfoDictionaryKey:@"CFBundleShortVersionString"] isEqual:version]){
 [fm removeItemAtPath:stage error:nil];updateFailure(@"The downloaded app failed verification.");return;}
 NSTask *helper=[NSTask new];helper.executableURL=[NSURL fileURLWithPath:@"/bin/bash"];
 helper.arguments=@[[app stringByAppendingPathComponent:@"Contents/Resources/mac-install.sh"],[@(NSProcessInfo.processInfo.processIdentifier) stringValue],stage,dest];
 if(![helper launchAndReturnError:nil]){[fm removeItemAtPath:stage error:nil];updateFailure(@"The update helper could not start.");return;}
 dispatch_async(dispatch_get_main_queue(),^{[NSApp terminate:nil];});
 }] resume];
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
 dispatch_async(dispatch_get_main_queue(),^{NSAlert *a=[NSAlert new];a.messageText=[NSString stringWithFormat:@"Update to RMP %@?",best];a.informativeText=@"RMP will download the update and reopen.";
 NSScrollView *scroll=[[NSScrollView alloc]initWithFrame:NSMakeRect(0,0,400,160)];scroll.hasVerticalScroller=YES;NSTextView *text=[[NSTextView alloc]initWithFrame:scroll.bounds];text.editable=NO;text.string=notes;scroll.documentView=text;a.accessoryView=scroll;
 [a addButtonWithTitle:@"Update"];[a addButtonWithTitle:@"Not now"];NSModalResponse answer=[a runModal];checkingUpdates=NO;explicitCheck=NO;if(answer==NSAlertFirstButtonReturn)stageUpdate(asset,best);
 });
 }] resume];
}
