// Run on macOS from the repository root to regenerate the Windows transport artwork.
#import <Cocoa/Cocoa.h>
#import "../mac-theme.h"
int main(void){@autoreleasepool{
 [NSApplication sharedApplication];
 NSArray *buttons=@[@[@"play",@"Play",@"toggle:",@34],@[@"pause",@"Pause",@"toggle:",@34],
 @[@"prev",@"Previous",@"prev:",@70],@[@"stop",@"■",@"stopPlayback:",@30],@[@"next",@"Next",@"next:",@46],
 @[@"full",@"⛶",@"full:",@30],@[@"queue",@"Queue",@"queuePopup:",@58],@[@"repeat",@"↻",@"repeatToggle:",@30],@[@"shuffle",@"⤨",@"shuffleToggle:",@28]];
 for(NSArray *spec in buttons){
  TransportButton *b=[TransportButton buttonWithTitle:spec[1] target:nil action:NSSelectorFromString(spec[2])];
  b.skin=SEVEN;b.glassGlow=YES;b.bordered=NO;b.font=[NSFont systemFontOfSize:13];int width=[spec[3] intValue], height=30;
  b.frame=NSMakeRect(0,0,width,height);
  NSImage *image=[[NSImage alloc] initWithSize:NSMakeSize(width*3,height*3)];
  [image lockFocus];NSAffineTransform *scale=[NSAffineTransform transform];[scale scaleBy:3];[scale concat];
  [b displayRectIgnoringOpacity:b.bounds inContext:NSGraphicsContext.currentContext];[image unlockFocus];
  NSBitmapImageRep *bitmap=[NSBitmapImageRep imageRepWithData:image.TIFFRepresentation];
  NSData *png=[bitmap representationUsingType:NSBitmapImageFileTypePNG properties:@{}];
  if(![png writeToFile:[NSString stringWithFormat:@"assets/seven/%@.png",spec[0]] atomically:YES])return 1;
 }
}}
