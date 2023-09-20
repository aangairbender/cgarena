pub mod arena;
pub mod bot_service;

// Server functionality is split into services, which communicate by means of events (messages).
// Services:
// * BotService - service which manages bots, allows adding and removing bots (and maybe renaming in the future).
// * 


// Flow when we add a bot
// 1. User makes "add bot" request with API
// 2. File with bot source code is created
// 3. Bot details (id, name, language, etc) are persisted
// 4. Success response is sent to user (201 Accepted)
// -- async --
// 5. Source code build process is started locally
// 6. On success bot is marked as ready for matches


// Flow when we start a batch of matches between bots
// 1. User makes "run matches" request with API
// 2. Job "run matches" is created and persisted. Matches for the job are created and persisted (with some init status).
// 3. Success response is sent to user (201 Accepted)
// -- async --
// 4. Match with init status is picked up and assigned to some worker
// 5. Worker executes the match
// 6. When match is finished, results are persisted and job progress is updated.

// So following REST resources would be available through the api:
// 1. Bot
// 2. Match
// 3. Job
// 4. Worker (readonly? so only can set through the config)