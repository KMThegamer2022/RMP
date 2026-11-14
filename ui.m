#import <Cocoa/Cocoa.h>
#include <stdbool.h>
#import "mac-theme.h"
#import "updates/mac.h"
typedef struct Player Player;
extern void ap_stop(Player *);
extern Player *ap_create(void *);
extern void ap_destroy(Player *),ap_add(Player *,const char *,bool),ap_toggle(Player *),ap_seek(Player *,int64_t),ap_volume(Player *,int);
extern bool ap_play(Player *,size_t),ap_seekable(Player *),ap_select(Player *,bool,int);
extern long ap_index(Player *);
extern size_t ap_count(Player *);
extern const char *ap_item(Player *,size_t);
extern int ap_state(Player *);
extern int64_t ap_time(Player *),ap_length(Player *);
extern void ap_tracks(Player *,bool,void *,void(*)(void *,int,const char *,bool));

@interface PlayerApp : NSObject<NSApplicationDelegate,NSMenuDelegate,NSWindowDelegate>{
 Player *p;
 NSWindow *window;
 NSVisualEffectView *aeroDeck;
 NSView *screen;
 NSTextField *name,*status,*clockLabel,*welcome;
 NSSlider *seek,*volume;
 NSButton *play;
 NSMenu *queue,*audio,*subs,*themeMenu;
 NSInteger skin;BOOL inFull,repeatOne,shuffle;
 NSPanel *panel; NSButton *fp; NSSlider *fs,*fv;NSPoint lastPointer;double lastActivity;
 NSImageView *cone;NSStackView *top,*bottom,*timeline;
 NSLayoutConstraint *normalTop,*edgeTop,*normalBottom,*edgeBottom;

 NSMutableArray<NSURL *> *pending;
 NSTimer *timer;
 int previous;
}
@end

static NSString *stamp(int64_t ms){
 long s=MAX(0,ms)/1000;
 return s>=3600?[NSString stringWithFormat:@"%ld:%02ld:%02ld",s/3600,s/60%60,s%60]:[NSString stringWithFormat:@"%ld:%02ld",s/60,s%60];
}
static NSTextField *label(NSString *text,CGFloat size,BOOL bold){
 NSTextField *f=[NSTextField labelWithString:text];
 f.font=bold?[NSFont boldSystemFontOfSize:size]:[NSFont systemFontOfSize:size];
 f.lineBreakMode=NSLineBreakByTruncatingMiddle;
 return f;
}
@implementation PlayerApp
-(NSMenuItem *)item:(NSMenu *)m title:(NSString *)t action:(SEL)a key:(NSString *)key{
 NSMenuItem *i=[m addItemWithTitle:t action:a keyEquivalent:key];i.target=self;return i;
}
-(NSMenu *)menu:(NSString *)title parent:(NSMenu *)parent{
 NSMenuItem *item=[parent addItemWithTitle:title action:nil keyEquivalent:@""];
 NSMenu *m=[[NSMenu alloc]initWithTitle:title];item.submenu=m;return m;
}
-(void)manualUpdate:(id)sender{checkMacUpdates(YES);}
-(void)menus{
 NSMenu *bar=[NSMenu new];NSApp.mainMenu=bar;
 NSMenu *app=[self menu:@"RMP" parent:bar];
 [self item:app title:@"Check for Updates…" action:@selector(manualUpdate:) key:@""];
 [app addItem:NSMenuItem.separatorItem];
 [app addItemWithTitle:@"Quit RMP" action:@selector(terminate:) keyEquivalent:@"q"];
 NSMenu *file=[self menu:@"File" parent:bar];
 [self item:file title:@"Open Files…" action:@selector(open:) key:@"o"];
 [self item:file title:@"Open Network Stream…" action:@selector(stream:) key:@"u"];
 NSMenu *edit=[self menu:@"Edit" parent:bar];
 for(NSString *s in @[@"Cut",@"Copy",@"Paste",@"Select All"]){
 NSArray *keys=@[@"x",@"c",@"v",@"a"]; NSUInteger n=edit.numberOfItems;
 [edit addItemWithTitle:s action:NSSelectorFromString(@[@"cut:",@"copy:",@"paste:",@"selectAll:"][n]) keyEquivalent:keys[n]];
 }
 NSMenu *media=[self menu:@"Playback" parent:bar];
 [self item:media title:@"Play / Pause" action:@selector(toggle:) key:@" "].keyEquivalentModifierMask=0;
 [self item:media title:@"Previous Item" action:@selector(prev:) key:@"["];
 [self item:media title:@"Next Item" action:@selector(next:) key:@"]"];
 [self item:media title:@"Toggle Full Screen" action:@selector(full:) key:@"f"].keyEquivalentModifierMask=NSEventModifierFlagControl|NSEventModifierFlagCommand;
 queue=[self menu:@"Queue" parent:bar];audio=[self menu:@"Audio Track" parent:bar];subs=[self menu:@"Subtitles" parent:bar];
 themeMenu=[self menu:@"Theme" parent:bar];
 for(NSString *title in @[@"VLC",@"RMP Default",@"Windows 10/11",@"Windows 7",@"Windows XP",@"Mac"]){
 NSMenuItem *entry=[self item:themeMenu title:title action:@selector(pickTheme:) key:@""];entry.tag=themeMenu.numberOfItems==1?VLC_THEME:themeMenu.numberOfItems==2?RMP_DEFAULT:themeMenu.numberOfItems-3;
 }
 queue.delegate=self;audio.delegate=self;subs.delegate=self;
}
-(NSButton *)button:(NSString *)title action:(SEL)action{
 TransportButton *b=[TransportButton buttonWithTitle:title target:self action:action];b.bordered=NO;return b;
}
-(void)styleView:(NSView *)v {
 Theme t=themes[skin];
 if([v isKindOfClass:TransportButton.class]){
 TransportButton *b=(TransportButton *)v;b.skin=skin;b.toolTip=b.title;b.font=[NSFont systemFontOfSize:13 weight:skin==MODERN?NSFontWeightMedium:NSFontWeightRegular];[b setNeedsDisplay:YES];
 }else if([v isKindOfClass:NSTextField.class] && v!=welcome){((NSTextField *)v).textColor=rgb(t.text);}
 if(v!=screen)for(NSView *child in v.subviews)[self styleView:child];
}
-(void)arrangeChrome {
 BOOL edge=inFull||skin==VLC_THEME;
 normalTop.active=NO;edgeTop.active=NO;normalBottom.active=NO;edgeBottom.active=NO;
 (edge?edgeTop:normalTop).active=YES;(inFull?edgeBottom:normalBottom).active=YES;
 top.hidden=edge;timeline.hidden=inFull;bottom.hidden=inFull;
 cone.hidden=skin!=VLC_THEME||(p&&ap_index(p)>=0);welcome.hidden=skin==VLC_THEME||(p&&ap_index(p)>=0);
}
-(void)applyTheme:(NSInteger)value {
 skin=value>=0 && value<THEME_COUNT?value:VLC_THEME;
 ThemeSurface *surface=(ThemeSurface *)window.contentView;surface.skin=skin;[surface setNeedsDisplay:YES];
 BOOL translucent=skin==SEVEN && !NSWorkspace.sharedWorkspace.accessibilityDisplayShouldReduceTransparency;
 aeroDeck.hidden=!translucent;window.opaque=!translucent;
 window.appearance=[NSAppearance appearanceNamed:(skin==MAC||skin==RMP_DEFAULT)?NSAppearanceNameDarkAqua:NSAppearanceNameAqua];
 window.titlebarAppearsTransparent=YES;
 window.backgroundColor=translucent?NSColor.clearColor:rgb(skin==XP?0x3978db:skin==MODERN?0xf5f5f5:themes[skin].panel);
 [self styleView:surface];[self arrangeChrome];if(panel)[self styleView:panel.contentView];
 if(skin==XP || skin==MODERN){NSColor *ink=skin==XP?NSColor.whiteColor:rgb(0x181818);name.textColor=ink;status.textColor=ink;}
 screen.layer.borderWidth=skin==XP?4:skin==SEVEN?2:0;
 screen.layer.borderColor=rgb(skin==XP?0x1256ce:0x829db5).CGColor;
 ((VolumeSlider *)volume).wedge=skin==VLC_THEME;[volume setNeedsDisplay:YES];
 seek.trackFillColor=rgb(themes[skin].accent);volume.trackFillColor=rgb(themes[skin].accent);
 for(NSView *v in surface.subviews)if([v isKindOfClass:NSStackView.class] && [((NSStackView *)v).arrangedSubviews containsObject:play]){
 NSStackView *row=(NSStackView *)v;row.spacing=skin==SEVEN?2:skin==MODERN?18:8;
 }

 for(NSMenuItem *entry in themeMenu.itemArray)entry.state=entry.tag==skin?NSControlStateValueOn:NSControlStateValueOff;
}
-(void)pickTheme:(NSMenuItem *)entry {
 [self applyTheme:entry.tag];[NSUserDefaults.standardUserDefaults setInteger:skin forKey:@"PlayerTheme"];
}
-(NSStackView *)row:(NSArray *)views{
 NSStackView *r=[NSStackView stackViewWithViews:views];r.spacing=10;r.orientation=NSUserInterfaceLayoutOrientationHorizontal;return r;
}
-(void)applicationDidFinishLaunching:(NSNotification *)note{
 [self menus];NSApp.appearance=[NSAppearance appearanceNamed:NSAppearanceNameDarkAqua];
 window=[[NSWindow alloc]initWithContentRect:NSMakeRect(0,0,900,570) styleMask:NSWindowStyleMaskTitled|NSWindowStyleMaskClosable|NSWindowStyleMaskResizable|NSWindowStyleMaskMiniaturizable backing:NSBackingStoreBuffered defer:NO];
 window.title=@"RMP";window.minSize=NSMakeSize(720,380);window.delegate=self;
 window.collectionBehavior=NSWindowCollectionBehaviorFullScreenPrimary;[window center];
 window.contentView=[[ThemeSurface alloc]initWithFrame:window.contentView.bounds];NSView *root=window.contentView;root.wantsLayer=YES;
 screen=[NSView new];screen.wantsLayer=YES;screen.layer.backgroundColor=NSColor.blackColor.CGColor;
 [NSEvent addLocalMonitorForEventsMatchingMask:NSEventMaskLeftMouseDown handler:^NSEvent *(NSEvent *event){
  if(event.window==self->window && event.clickCount==2){
   NSPoint point=[self->screen convertPoint:event.locationInWindow fromView:nil];
   if(NSPointInRect(point,self->screen.bounds)){[self full:nil];return nil;}
  }
  return event;
 }];
 cone=[NSImageView new];cone.image=[NSImage imageNamed:@"vlc-cone"];[screen addSubview:cone];cone.translatesAutoresizingMaskIntoConstraints=NO;
 [NSLayoutConstraint activateConstraints:@[[cone.centerXAnchor constraintEqualToAnchor:screen.centerXAnchor],[cone.centerYAnchor constraintEqualToAnchor:screen.centerYAnchor],[cone.widthAnchor constraintEqualToConstant:120],[cone.heightAnchor constraintEqualToConstant:120]]];
 welcome=label(@"RMP",42,YES);welcome.textColor=[NSColor colorWithWhite:0.65 alpha:1];
 [screen addSubview:welcome];welcome.translatesAutoresizingMaskIntoConstraints=NO;
 [welcome.centerXAnchor constraintEqualToAnchor:screen.centerXAnchor].active=YES;
 [welcome.centerYAnchor constraintEqualToAnchor:screen.centerYAnchor].active=YES;
 name=label(@"RMP",18,YES);status=label(@"Ready",12,NO);
 [name setContentCompressionResistancePriority:200 forOrientation:NSLayoutConstraintOrientationHorizontal];
 NSView *space=[NSView new];
 top=[self row:@[name,space,status,[self button:@"Open Files…" action:@selector(open:)]]];
 play=[self button:@"Play" action:@selector(toggle:)];
 volume=[VolumeSlider sliderWithValue:80 minValue:0 maxValue:100 target:self action:@selector(changeVolume:)];volume.toolTip=@"Volume";
 [volume.widthAnchor constraintEqualToConstant:85].active=YES;
 seek=[NSSlider sliderWithValue:0 minValue:0 maxValue:1 target:self action:@selector(scrub:)];seek.continuous=NO;seek.toolTip=@"Playback position";
 clockLabel=label(@"0:00 / 0:00",12,NO);clockLabel.font=[NSFont monospacedDigitSystemFontOfSize:12 weight:NSFontWeightRegular];
 timeline=[self row:@[seek,clockLabel]];
 bottom=[self row:@[play,[self button:@"Previous" action:@selector(prev:)],[self button:@"■" action:@selector(stopPlayback:)],[self button:@"Next" action:@selector(next:)],[self button:@"⛶" action:@selector(full:)],[self button:@"Queue" action:@selector(queuePopup:)],[self button:@"↻" action:@selector(repeatToggle:)],[self button:@"⤨" action:@selector(shuffleToggle:)],[NSView new],label(@"Volume",12,NO),volume]];
 for(NSView *v in @[top,screen,timeline,bottom]){
 [root addSubview:v];v.translatesAutoresizingMaskIntoConstraints=NO;
 CGFloat inset=v==screen?0:16;
 [v.leadingAnchor constraintEqualToAnchor:root.leadingAnchor constant:inset].active=YES;[v.trailingAnchor constraintEqualToAnchor:root.trailingAnchor constant:-inset].active=YES;
 }
 [top.topAnchor constraintEqualToAnchor:root.topAnchor constant:10].active=YES;
 normalTop=[screen.topAnchor constraintEqualToAnchor:top.bottomAnchor constant:10];edgeTop=[screen.topAnchor constraintEqualToAnchor:root.topAnchor];
 normalBottom=[timeline.topAnchor constraintEqualToAnchor:screen.bottomAnchor constant:6];edgeBottom=[screen.bottomAnchor constraintEqualToAnchor:root.bottomAnchor];
 [bottom.topAnchor constraintEqualToAnchor:timeline.bottomAnchor constant:6].active=YES;
 [bottom.bottomAnchor constraintEqualToAnchor:root.bottomAnchor constant:-12].active=YES;
 aeroDeck=[[NSVisualEffectView alloc]initWithFrame:NSZeroRect];aeroDeck.material=NSVisualEffectMaterialLight;
 aeroDeck.blendingMode=NSVisualEffectBlendingModeBehindWindow;aeroDeck.state=NSVisualEffectStateActive;
 [root addSubview:aeroDeck positioned:NSWindowBelow relativeTo:timeline];aeroDeck.translatesAutoresizingMaskIntoConstraints=NO;
 [NSLayoutConstraint activateConstraints:@[[aeroDeck.leadingAnchor constraintEqualToAnchor:root.leadingAnchor],[aeroDeck.trailingAnchor constraintEqualToAnchor:root.trailingAnchor],[aeroDeck.bottomAnchor constraintEqualToAnchor:root.bottomAnchor],[aeroDeck.topAnchor constraintEqualToAnchor:screen.bottomAnchor]]];
 [NSUserDefaults.standardUserDefaults registerDefaults:@{@"PlayerTheme":@(VLC_THEME)}];
 [self applyTheme:[NSUserDefaults.standardUserDefaults integerForKey:@"PlayerTheme"]];
 p=ap_create((__bridge void *)screen);
 if(!p){NSAlert *a=[NSAlert new];a.messageText=@"RMP could not load its playback engine.";[a runModal];[NSApp terminate:nil];return;}
 [window makeKeyAndOrderFront:nil];[NSApp activateIgnoringOtherApps:YES];
 timer=[NSTimer scheduledTimerWithTimeInterval:0.2 target:self selector:@selector(refresh:) userInfo:nil repeats:YES];
 if(pending.count){[self accept:pending];pending=nil;}
 [self refresh:nil];
 checkMacUpdates(NO);
}
-(void)application:(NSApplication *)app openURLs:(NSArray<NSURL *> *)urls{
 if(p)[self accept:urls];else{if(!pending)pending=[NSMutableArray new];[pending addObjectsFromArray:urls];}
}
-(void)accept:(NSArray<NSURL *> *)urls{
 size_t start=ap_count(p);
 for(NSURL *u in urls)ap_add(p,u.isFileURL?u.fileSystemRepresentation:u.absoluteString.UTF8String,!u.isFileURL);
 if(urls.count)[self launch:start];
}
-(NSString *)display:(size_t)i{
 NSString *s=[NSString stringWithUTF8String:ap_item(p,i)];
 return s.lastPathComponent.length?s.lastPathComponent:s;
}
-(void)launch:(size_t)i{
 if(i>=ap_count(p))return;
 previous=-1;welcome.hidden=YES;cone.hidden=YES;
 name.stringValue=[self display:i];window.title=[@"RMP — " stringByAppendingString:name.stringValue];
 status.stringValue=ap_play(p,i)?@"Opening…":@"Could not open media";
}
-(void)refresh:(id)sender{
 int state=ap_state(p);
 if(state==6 && previous!=6 && repeatOne){[self launch:ap_index(p)];return;}
 if(state==6 && previous!=6 && (shuffle||ap_index(p)+1<(long)ap_count(p))){[self next:nil];return;}
 previous=state;
 NSArray *states=@[@"Ready",@"Opening…",@"Buffering…",@"Playing",@"Paused",@"Stopped",@"Finished",@"Playback error"];
 status.stringValue=states[MAX(0,MIN(7,state))];play.title=state>=1&&state<=3?@"Pause":@"Play";
 int64_t t=ap_time(p),d=ap_length(p);seek.enabled=ap_seekable(p)&&d>0;
 seek.doubleValue=d>0?(double)t/d:0;clockLabel.stringValue=[NSString stringWithFormat:@"%@ / %@",stamp(t),d>0?stamp(d):@"—"];[self refreshFloating];
}
-(void)open:(id)s{
 NSOpenPanel *picker=[NSOpenPanel openPanel];picker.allowsMultipleSelection=YES;
 if([picker runModal]==NSModalResponseOK)[self accept:picker.URLs];
}
-(void)stream:(id)s{
 NSAlert *a=[NSAlert new];a.messageText=@"Open Network Stream";[a addButtonWithTitle:@"Open"];[a addButtonWithTitle:@"Cancel"];
 NSTextField *f=[[NSTextField alloc]initWithFrame:NSMakeRect(0,0,400,24)];f.placeholderString=@"https://";a.accessoryView=f;a.window.initialFirstResponder=f;
 if([a runModal]!=NSAlertFirstButtonReturn)return;
 NSURL *url=[NSURL URLWithString:[f.stringValue stringByTrimmingCharactersInSet:NSCharacterSet.whitespaceAndNewlineCharacterSet]];
 if(url.scheme.length && !url.isFileURL){[self accept:@[url]];return;}
 NSAlert *err=[NSAlert new];err.messageText=@"Enter a stream URL including its protocol.";[err runModal];
}
-(void)prev:(id)s{if(ap_index(p)>0)[self launch:ap_index(p)-1];else if(ap_count(p))[self launch:0];}
-(void)next:(id)s{NSUInteger n=ap_count(p);if(shuffle && n>1)[self launch:(ap_index(p)+1+arc4random_uniform((uint32_t)n-1))%n];else [self launch:ap_index(p)+1];}
-(void)repeatToggle:(NSButton *)b{repeatOne=!repeatOne;b.title=repeatOne?@"↻ 1":@"↻";b.toolTip=@"Repeat current item";}
-(void)shuffleToggle:(NSButton *)b{shuffle=!shuffle;b.title=shuffle?@"⤨ ✓":@"⤨";b.toolTip=@"Shuffle queue";}
-(void)toggle:(id)s{if(!ap_count(p))[self open:s];else ap_toggle(p);}
#include "mac-fullscreen.h"
-(void)stopPlayback:(id)s{ap_stop(p);}
-(void)queuePopup:(NSButton *)s{[queue popUpMenuPositioningItem:nil atLocation:NSMakePoint(0,s.bounds.size.height) inView:s];}
-(void)full:(id)s{[window toggleFullScreen:nil];}
-(void)scrub:(id)s{ap_seek(p,seek.doubleValue*ap_length(p));}
-(void)changeVolume:(id)s{ap_volume(p,volume.intValue);}
-(void)choose:(NSMenuItem *)i{if(i.menu==queue)[self launch:i.tag];else if(!ap_select(p,i.menu==subs,(int)i.tag))NSBeep();}
static void addTrack(void *context,int idNum,const char *title,bool active){
 NSMenu *m=(__bridge NSMenu *)context; NSMenuItem *i=[m addItemWithTitle:[NSString stringWithUTF8String:title]?:@"Track" action:@selector(choose:) keyEquivalent:@""];
 i.target=m.delegate;i.tag=idNum;i.state=active?NSControlStateValueOn:NSControlStateValueOff;
}
-(void)menuNeedsUpdate:(NSMenu *)m{
 [m removeAllItems];if(!p)return;
 if(m==queue){for(size_t n=0;n<ap_count(p);n++)addTrack((__bridge void *)m,(int)n,[self display:n].UTF8String,(long)n==ap_index(p));}
 else ap_tracks(p,m==subs,(__bridge void *)m,addTrack);
 if(!m.numberOfItems)[m addItemWithTitle:m==queue?@"Queue is empty":@"No tracks available" action:nil keyEquivalent:@""];
}
-(BOOL)applicationShouldTerminateAfterLastWindowClosed:(NSApplication *)a{return YES;}
-(void)applicationWillTerminate:(NSNotification *)n{[timer invalidate];if(p)ap_destroy(p);}
@end
int main(void){@autoreleasepool{
 NSString *plugins=[NSBundle.mainBundle.executablePath.stringByDeletingLastPathComponent stringByAppendingPathComponent:@"plugins"];
 setenv("VLC_PLUGIN_PATH",plugins.fileSystemRepresentation,1);
 NSApplication *app=NSApplication.sharedApplication;[app setActivationPolicy:NSApplicationActivationPolicyRegular];
 __attribute__((objc_precise_lifetime)) PlayerApp *delegate=[PlayerApp new];app.delegate=delegate;[app run];(void)delegate;
}return 0;}
