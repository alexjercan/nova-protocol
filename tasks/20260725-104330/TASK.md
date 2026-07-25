# user feedback session for v0.9.0

- STATUS: OPEN
- PRIORITY: 0
- TAGS: v0.9.0,feedback,gameplay,ui,ux

Feedback Ideas for the Game Nova Protocol

The Main Menu + UI elements in this scene
- the UI feels a bit "washed" it has weak lines and it's definetly prototype phase
- the font in the UI needs to be changed to something better - I would like a monospace terminal font
- shade of blue for background doesn't look that good anymore in my opinion
- I would like to have a more "terminal" "spaceship HUD" style to the UI elements
- maybe even changing the color scheme a bit (both in app and in game)
- the UI screens we open should have fixed sizing not chaning based on text
- proper settings menu with multiple panels for each category + more settings for different things
- mods lists and scenarios lists should be scrollable
- scenario art to be used more as a background blurred on top image in the scenario picker
- make the scenario picker (also the mod explorer) windows a bit smaller + more terminal TUI look to them
- the Terminal TUI look might even include the classic "X" button at the top like Windows windows also borders
- but I like square borders, not aero style, sharp style kind of - I am not that sold on the current color for the background
- the Palette is configurable from what I can tell, maybe even modable (but let's not go that far yet) - the good thing is that colors are all in one place so it might be easy to test
- Positive: I like how the "Nova Protocol - line - buttons" style looks like -> but again more towards a TUI style - but the layout is nice in that small UI menu - very minimalistic, if it also had a terminal TUI/kitty/bash CLI feeling to it, it would be perfect

For the gameplay (it will include mainline suggestions only, mods are mods and they should work on themselves by themselves)
- Chat Bubbles are very small - we should make some improvements there (I know we added "icons" but again, more terminal vibe is what I would like -> scp/curl/tcp transmission kind of -> plus you see it in the flight logs in "Tab" mode)
- I think we might need a space station in the first scenario to fit with the chat conversation - we are leaving a spacestation
- maybe we need a keybind to skip chat messages (like press Enter to skip or something so that you do not wait the full 5 seconds of them appearing)
- the initial 5 messages in the game are pretty boring -> we can try to implement something that allows "cut scenes" to have the player spaceship do something (especially if we would have a station) like we de-dock (or how you say it)
- maybe we even introduce the "SHIFT" RCS mechanic initially to let the player get out of the station - but not sold on it
- but a docked ship with cinematic view (basically in docked mode I can say it's just like orbit mode you see the ship from a distance) would sound interesting + making the messages faster/skipable (faster based on length) + maybe a sound for them to at least have something (we have a beep, but some animation or something - if it's terminal style I can see an idea of having a typewriter effect)
- and also while the convo is going it would be cool to have the ship "pupetted" to exit the docking area (not by player) but as a cutscene
- picking up the salvage crates makes a sound but I feel like I need more feedback some kind of "+1 text" or somehow updating the objective
- the big in the middle objective that appears when you get a new objective might be too big and in the middle, maybe it would fit better closer to the edge and staying on screen until you open tab "to move it in" - like accepting it or something like that, but it disapears if you complete it before you Tab
- so the objective would have smaller size, but still have this kind of animation of tweening smaller and rotating from the sphere to the corner of the screen in top right + it should already be closer to the top right if not there already
- tab drawer same as everything so far -> more terminal style + I think we might want a small padding between the edge of the tabs and the screen - right now it's right on the edge
- also didn't mention it but overall text on the screen (when AUTO pilot is kind of too much I think - but really don't know what to do with it because it is useful and cool (the ETA xs | ym) and the (FLIP) and (GOTO | BURN) but not sure how to make it less, really needs it's own exploration phase
- also the objective completed green text still appears that's a bug
- I also think that we should have an edge to the map, something that if you go past you get some kind of screen like it's losing signal and you need to go back + maybe it even slows you down until you get to zero speed unless you go towards the map center + enemies that are damaged and go into the border get despawned for example, or things in general (shards + asteroids pieces)
- Scenario 2: I feel like the two corvettes in the second scenario appear out of the blue + maybe we can have more asteroids in the scene and they are already spawned in but such that we cannot really see them + they activate in an ambush style so they wait for you to get close and they attack (maybe even marked as neutral until that point - so you don't really know they are enemies)
- Scenario 3: feels good; just keeping the same scene style in + maybe we spawn near the ceres queen + the tally spawns further away (maybe where we spawn initially - like switch places because we already found and defended the ceres queen)
- noticed issue with multiple objectives at the same time - they need to be away and appear in different places
- Scenario 4: instead of beacons we might want to have actual space stations build by pieces and floating around that would be cool + maybe neutral / maybe friendly IDK, they won't have guns anyway
- sometimes the combat radar loses focus randomly - not sure if it's gameplay or bug but it's annoying
- Scenario 5: it's a bit weird you hold lock and instantly get a next convo that the tally is there, doesn't really make sense I would do it in another way: maybe more asteroids it's harder to navigate and you have to investigate something liek a ruined based or something and then enemies would come in a single wave from "a raid" like they are coming back home with suplies from a raid or something and you were investigating there and cought by surpise and have to fight them to escape -> but the ending is kind of meh then because you just escape not really finish them, but maybe that's the story IDK or maybe we will add another scenario or something like that
