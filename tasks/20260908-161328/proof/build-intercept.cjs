// Static builds only. This never creates a development server.
const path=require('node:path');
const root=path.resolve(__dirname,'../../..');process.chdir(path.join(root,'web'));
// This unused preset bypasses the serving configuration's free-port probe.
process.env.NOVA_UI_PORT='1';
const webpack=require(path.join(root,'web/node_modules/webpack'));
(async()=>{
 const kind=process.argv[2]||'local';
 const config=await require(path.join(root,'web/webpack.config.js'))({WEBPACK_SERVE:kind==='local'},{mode:kind==='local'?'development':'production'});
 config.output.path=path.join(root,'web/.cache/story/intercept',`${kind}${process.env.PUBLIC_PATH?'-prefix':''}`);
 const compiler=webpack(config);
 compiler.run((error,stats)=>compiler.close(()=>{if(error)throw error;console.log(stats.toString({colors:false}));process.exitCode=stats.hasErrors()?1:0;}));
})();
