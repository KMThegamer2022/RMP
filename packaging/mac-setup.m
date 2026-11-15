#import <Cocoa/Cocoa.h>
int main(void){@autoreleasepool{
 [NSApplication sharedApplication];[NSApp setActivationPolicy:NSApplicationActivationPolicyRegular];[NSApp activateIgnoringOtherApps:YES];
 NSAlert *a=[NSAlert new];a.messageText=@"Install Rust Media Player";
 a.informativeText=@"Install RMP in your Applications folder. Choose one theme to start with. You can change it anytime under View → Theme in the macOS menu bar.";
 NSArray *names=@[@"VLC",@"RMP Default",@"Windows 10/11",@"Windows 7",@"Windows XP",@"Mac",@"Linux"];
 NSArray<NSNumber *> *tags=@[@5,@4,@0,@1,@2,@3,@6];
 NSView *options=[[NSView alloc]initWithFrame:NSMakeRect(0,0,300,183)];
 NSDictionary *prefs=[NSUserDefaults.standardUserDefaults persistentDomainForName:@"io.github.kmthegamer2022.rmp"];
 NSNumber *stored=prefs[@"PlayerTheme"];NSInteger selected=stored && stored.integerValue>=0 && stored.integerValue<(NSInteger)tags.count?stored.integerValue:5;
 NSMutableArray *buttons=[NSMutableArray new];
 for(NSInteger i=0;i<(NSInteger)tags.count;i++){NSButton *b=[NSButton radioButtonWithTitle:names[i] target:nil action:@selector(self)];b.frame=NSMakeRect(0,156-i*25,280,24);b.tag=tags[i].integerValue;b.state=b.tag==selected;[options addSubview:b];[buttons addObject:b];}
 a.accessoryView=options;[a addButtonWithTitle:@"Install"];[a addButtonWithTitle:@"Cancel"];if([a runModal]!=NSAlertFirstButtonReturn)return 0;
 NSFileManager *fm=NSFileManager.defaultManager;NSString *root=[NSHomeDirectory() stringByAppendingPathComponent:@"Applications"];
 NSString *dest=[root stringByAppendingPathComponent:@"RMP.app"];
 for(NSRunningApplication *r in [NSRunningApplication runningApplicationsWithBundleIdentifier:@"io.github.kmthegamer2022.rmp"]){if([r.bundleURL.path isEqual:dest]){NSAlert *e=[NSAlert new];e.messageText=@"Quit the running RMP player, then open Setup again.";[e runModal];return 1;}}
 NSError *err=nil;NSString *src=[NSBundle.mainBundle.resourcePath stringByAppendingPathComponent:@"RMP.app"];NSString *stage=[root stringByAppendingPathComponent:[@".RMP-" stringByAppendingString:NSUUID.UUID.UUIDString]];
 if([fm createDirectoryAtPath:root withIntermediateDirectories:YES attributes:nil error:&err] && [fm copyItemAtPath:src toPath:stage error:&err]){
 if([fm fileExistsAtPath:dest]) [fm replaceItemAtURL:[NSURL fileURLWithPath:dest] withItemAtURL:[NSURL fileURLWithPath:stage] backupItemName:nil options:0 resultingItemURL:nil error:&err];
 else [fm moveItemAtPath:stage toPath:dest error:&err];}
 if(err){[fm removeItemAtPath:stage error:nil];[[NSAlert alertWithError:err]runModal];return 1;}
 for(NSButton *b in buttons)if(b.state==NSControlStateValueOn){if(b.tag!=6 || selected!=6)CFPreferencesSetAppValue(CFSTR("PreviousTheme"),(__bridge CFNumberRef)@(b.tag==6?selected:b.tag),CFSTR("io.github.kmthegamer2022.rmp"));
 CFPreferencesSetAppValue(CFSTR("PlayerTheme"),(__bridge CFNumberRef)@(b.tag),CFSTR("io.github.kmthegamer2022.rmp"));CFPreferencesAppSynchronize(CFSTR("io.github.kmthegamer2022.rmp"));break;}
 [NSWorkspace.sharedWorkspace openURL:[NSURL fileURLWithPath:dest]];return 0;
}}
