var searchIndex = JSON.parse('{\
"cache_manager":{"doc":"","t":[3,4,3,13,13,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,12],"n":["Cache","CacheAction","CacheManager","Close","Used","borrow","borrow","borrow","borrow_mut","borrow_mut","borrow_mut","clone","clone","clone_into","clone_into","enforce_cache_size","fairing","flush_and_destroy","fmt","fmt","fmt","from","from","from","from_request","into","into","into","into_collection","into_collection","into_collection","mapped","mapped","mapped","new","to_owned","to_owned","try_from","try_from","try_from","try_into","try_into","try_into","type_id","type_id","type_id","used","vzip","vzip","vzip","0"],"q":["cache_manager","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","cache_manager::CacheAction"],"d":["A cache driver which is spawned off by the fairing for …","","Handles the size and implementation of the cache folder …","","","","","","","","","","","","","If the cache is greater than the maximum allowed size, …","","Attempt to close the cache, and destroy it. This will …","","","","","","","","","","","","","","","","","","","","","","","","","","","","","New Item created/used, and this should be reflected by the …","","","",""],"i":[0,0,0,1,1,2,1,3,2,1,3,1,3,1,3,2,3,3,2,1,3,2,1,3,3,2,1,3,2,1,3,2,1,3,2,1,3,2,1,3,2,1,3,2,1,3,3,2,1,3,4],"f":[null,null,null,null,null,[[]],[[]],[[]],[[]],[[]],[[]],[[],["cacheaction",4]],[[],["cache",3]],[[]],[[]],[[]],[[],["adhoc",3]],[[]],[[["formatter",3]],["result",6]],[[["formatter",3]],["result",6]],[[["formatter",3]],["result",6]],[[]],[[]],[[]],[[["request",3]],["pin",3,[["box",3,[["future",8]]]]]],[[]],[[]],[[]],[[],["smallvec",3]],[[],["smallvec",3]],[[],["smallvec",3]],[[],["smallvec",3]],[[],["smallvec",3]],[[],["smallvec",3]],[[["pathbuf",3],["u64",15]],["result",4,[["box",3,[["error",8]]]]]],[[]],[[]],[[],["result",4]],[[],["result",4]],[[],["result",4]],[[],["result",4]],[[],["result",4]],[[],["result",4]],[[],["typeid",3]],[[],["typeid",3]],[[],["typeid",3]],[[["pathbuf",3]]],[[]],[[]],[[]],null],"p":[[4,"CacheAction"],[3,"CacheManager"],[3,"Cache"],[13,"Used"]]},\
"config":{"doc":"","t":[11,11,11,11,11,3,13,13,11,4,11,11,11,13,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11],"n":["ALLOWED_CHARS","ALLOWED_FORMATS","API_NAME","BLACKLISTED_PHRASES","CACHE_PATH","Config","General","Langs","MAX_CACHE_SIZE","PathType","SPEED_MAX_VAL","SPEED_MIN_VAL","SUPPORTED_LANGS","Users","WORD_LENGTH_LIMIT","borrow","borrow","borrow_mut","borrow_mut","fairing","from","from","get_path","into","into","into_collection","into_collection","mapped","mapped","new","try_from","try_from","try_into","try_into","type_id","type_id","vzip","vzip"],"q":["config","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","",""],"d":["","","","","","","","","","The different config paths we can load from","","","","","","","","","","","","","","","","","","","","","","","","","","","",""],"i":[1,1,1,1,1,0,2,2,1,0,1,1,1,2,1,2,1,2,1,1,2,1,2,2,1,2,1,2,1,1,2,1,2,1,2,1,2,1],"f":[[[],["hashset",3]],[[],["hashset",3]],[[],["str",15]],[[]],[[],["str",15]],null,null,null,[[],["usize",15]],null,[[],["f32",15]],[[],["f32",15]],[[],["hashmap",3]],null,[[],["usize",15]],[[]],[[]],[[]],[[]],[[],["adhoc",3]],[[]],[[]],[[["pathbuf",3]],["pathbuf",3]],[[]],[[]],[[],["smallvec",3]],[[],["smallvec",3]],[[],["smallvec",3]],[[],["smallvec",3]],[[["pathbuf",3]],["result",4,[["configerror",4]]]],[[],["result",4]],[[],["result",4]],[[],["result",4]],[[],["result",4]],[[],["typeid",3]],[[],["typeid",3]],[[]],[[]]],"p":[[3,"Config"],[4,"PathType"]]},\
"converter":{"doc":"This library handles conversions from one file format to …","t":[4,3,8,3,13,13,13,13,13,11,11,11,11,11,11,10,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,11,10,11,11,11,10,11,11,11,11,11,11,11,11,11,11,11,11,11,11,12,12],"n":["ConversionError","Converter","ConverterSubprocess","Ffmpeg","IoFailure","NoExtension","NotFile","NotFound","Other","borrow","borrow","borrow","borrow_mut","borrow_mut","borrow_mut","convert","convert","convert","fairing","fmt","fmt","fmt","from","from","from","into","into","into","into_collection","into_collection","into_collection","is_supported","mapped","mapped","mapped","name","name","new","source","supported_outputs","supported_outputs","to_string","try_from","try_from","try_from","try_into","try_into","try_into","type_id","type_id","type_id","vzip","vzip","vzip","0","0"],"q":["converter","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","converter::ConversionError",""],"d":["","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","Create a new ffmpeg instance, this involves checking …","","","","","","","","","","","","","","","","","",""],"i":[0,0,0,0,1,1,1,1,1,2,1,3,2,1,3,4,2,3,2,1,1,3,2,1,3,2,1,3,2,1,3,2,2,1,3,4,3,3,1,4,3,1,2,1,3,2,1,3,2,1,3,2,1,3,5,6],"f":[null,null,null,null,null,null,null,null,null,[[]],[[]],[[]],[[]],[[]],[[]],[[["f32",15],["phrasepackage",3],["str",15],["config",3]],["pin",3,[["box",3,[["future",8]]]]]],[[["phrasepackage",3],["f32",15],["config",3]]],[[["f32",15],["phrasepackage",3],["str",15],["config",3]],["pin",3,[["box",3,[["future",8]]]]]],[[["vec",3,[["box",3,[["convertersubprocess",8]]]]]],["adhoc",3]],[[["formatter",3]],["result",6]],[[["formatter",3]],["result",6]],[[["formatter",3]],["result",6]],[[]],[[]],[[]],[[]],[[]],[[]],[[],["smallvec",3]],[[],["smallvec",3]],[[],["smallvec",3]],[[["string",3]],["bool",15]],[[],["smallvec",3]],[[],["smallvec",3]],[[],["smallvec",3]],[[],["str",15]],[[],["str",15]],[[],["result",4,[["string",3]]]],[[],["option",4,[["error",8]]]],[[],["hashset",3,[["string",3]]]],[[],["hashset",3,[["string",3]]]],[[],["string",3]],[[],["result",4]],[[],["result",4]],[[],["result",4]],[[],["result",4]],[[],["result",4]],[[],["result",4]],[[],["typeid",3]],[[],["typeid",3]],[[],["typeid",3]],[[]],[[]],[[]],null,null],"p":[[4,"ConversionError"],[3,"Converter"],[3,"Ffmpeg"],[8,"ConverterSubprocess"],[13,"Other"],[13,"IoFailure"]]},\
"festival_api":{"doc":"codecov Build Docs OAS Docs","t":[5,5,5,0,3,3,11,11,11,11,12,12,12,11,11,12,11,11,11,11,12,12,11,11,12,12,11,11,11,11,11,11,12,12,11,11,12,12],"n":["convert","index","main","models","GenerationRequest","NewGenerationRequest","borrow","borrow","borrow_mut","borrow_mut","crt","fmt","fmt","from","from","id","into","into","into_collection","into_collection","lang","lang","mapped","mapped","speed","speed","try_from","try_from","try_into","try_into","type_id","type_id","usr_id","usr_id","vzip","vzip","word","word"],"q":["festival_api","","","","festival_api::models","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","",""],"d":["Expects a phrase package, attempts to convert it to a …","The base url of the program. This is just a catch-all for …","","Various objects, including database objects, for the api.","A request to generate a .wav file from text from a user …","A request to generate a .wav file from text for a user, to …","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","",""],"i":[0,0,0,0,0,0,1,2,1,2,1,1,2,1,2,1,1,2,1,2,1,2,1,2,1,2,1,2,1,2,1,2,1,2,1,2,1,2],"f":[[[["json",3,[["phrasepackage",3]]],["flite",3],["converter",3],["config",3],["cache",3]]],[[["config",3]],["string",3]],[[]],null,null,null,[[]],[[]],[[]],[[]],null,null,null,[[]],[[]],null,[[]],[[]],[[],["smallvec",3]],[[],["smallvec",3]],null,null,[[],["smallvec",3]],[[],["smallvec",3]],null,null,[[],["result",4]],[[],["result",4]],[[],["result",4]],[[],["result",4]],[[],["typeid",3]],[[],["typeid",3]],null,null,[[]],[[]],null,null],"p":[[3,"GenerationRequest"],[3,"NewGenerationRequest"]]},\
"festvox":{"doc":"","t":[16,3,4,13,13,8,13,11,11,11,11,11,11,11,11,11,11,10,11,11,11,11,11,11,11,10,11,11,11,11,11,11,11,11,11,11,11,12,12,12],"n":["Error","Flite","FliteError","IoFailure","ProcessError","TtsGenerator","UnableToStart","borrow","borrow","borrow_mut","borrow_mut","fairing","fairing","fmt","fmt","from","from","generate","generate","into","into","into_collection","into_collection","mapped","mapped","new","new","source","to_string","try_from","try_from","try_into","try_into","type_id","type_id","vzip","vzip","0","0","0"],"q":["festvox","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","festvox::FliteError","",""],"d":["","","","","","A trait indicating a tts generator that can be constructed …","","","","","","Generate an adhoc fairing which can be bound to a …","Generate an adhoc fairing which can be bound to a …","","","","","Generate a phrase utilising the TTS system, with the set …","","","","","","","","Create a new TTS builder","","","","","","","","","","","","","",""],"i":[1,0,0,2,2,0,2,3,2,3,2,1,1,2,2,3,2,1,3,3,2,3,2,3,2,1,3,2,2,3,2,3,2,3,2,3,2,4,5,6],"f":[null,null,null,null,null,null,null,[[]],[[]],[[]],[[]],[[],["adhoc",3]],[[],["adhoc",3]],[[["formatter",3]],["result",6]],[[["formatter",3]],["result",6]],[[]],[[]],[[["phrasepackage",3],["config",3]],["pin",3,[["box",3,[["future",8]]]]]],[[["phrasepackage",3],["config",3]],["pin",3,[["box",3,[["future",8]]]]]],[[]],[[]],[[],["smallvec",3]],[[],["smallvec",3]],[[],["smallvec",3]],[[],["smallvec",3]],[[],["result",4]],[[],["result",4]],[[],["option",4,[["error",8]]]],[[],["string",3]],[[],["result",4]],[[],["result",4]],[[],["result",4]],[[],["result",4]],[[],["typeid",3]],[[],["typeid",3]],[[]],[[]],null,null,null],"p":[[8,"TtsGenerator"],[4,"FliteError"],[3,"Flite"],[13,"UnableToStart"],[13,"IoFailure"],[13,"ProcessError"]]},\
"macros":{"doc":"This module contains ease-of-use macros for the …","t":[14,14],"n":["failure","reject"],"q":["macros",""],"d":["A macro to shorthand the rejection from an endpoint due to …","A macro to shorthand the rejection from an endpoint due to …"],"i":[0,0],"f":[null,null],"p":[]},\
"response":{"doc":"","t":[3,13,13,4,13,13,11,11,11,11,11,12,11,11,11,11,11,11,11,11,11,11,11,11,12,11,11,11,11,11,11,11,11,12,12,12,12],"n":["Data","FileDownload","JsonOk","Response","TextErr","TextOk","borrow","borrow","borrow_mut","borrow_mut","data","data","fmt","fmt","from","from","into","into","into_collection","into_collection","mapped","mapped","respond_to","status","status","try_from","try_from","try_into","try_into","type_id","type_id","vzip","vzip","0","0","0","0"],"q":["response","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","","response::Response","","",""],"d":["Internal data that must be passed to a responder. Any data …","","","Represents a response from the api, the content-type and …","","","","","","","Returns the inner data of this response","","","","","","","","","","","","","Returns the status of this response","","","","","","","","","","","","",""],"i":[0,1,1,0,1,1,2,1,2,1,2,2,2,1,2,1,2,1,2,1,2,1,1,2,2,2,1,2,1,2,1,2,1,3,4,5,6],"f":[null,null,null,null,null,null,[[]],[[]],[[]],[[]],[[]],null,[[["formatter",3]],["result",6]],[[["formatter",3]],["result",6]],[[]],[[]],[[]],[[]],[[],["smallvec",3]],[[],["smallvec",3]],[[],["smallvec",3]],[[],["smallvec",3]],[[["request",3]],["result",6]],[[],["status",3]],null,[[],["result",4]],[[],["result",4]],[[],["result",4]],[[],["result",4]],[[],["typeid",3]],[[],["typeid",3]],[[]],[[]],null,null,null,null],"p":[[4,"Response"],[3,"Data"],[13,"TextErr"],[13,"TextOk"],[13,"JsonOk"],[13,"FileDownload"]]},\
"utils":{"doc":"","t":[5,0,5,0,3,11,11,11,11,11,12,11,11,11,12,11,12,11,11,11,11,11,12,3,11,11,11,11,11,11,11,11,11,11,11,11],"n":["generate_random_alphanumeric","phrase_package","sha_256_hash","test_utils","PhrasePackage","borrow","borrow_mut","deserialize","filename_stem_basespeed","filename_stem_properspeed","fmt","from","into","into_collection","lang","mapped","speed","try_from","try_into","type_id","validated","vzip","word","AlteredToml","borrow","borrow_mut","drop","from","into","into_collection","mapped","new","try_from","try_into","type_id","vzip"],"q":["utils","","","","utils::phrase_package","","","","","","","","","","","","","","","","","","","utils::test_utils","","","","","","","","","","","",""],"d":["Generate a randomised alphanumeric (base 62) string of a …","","Takes an input reference string, and hashes it using the …","","A phrase package which the user is requesting a speech to …","","","","Collect the name of the file pre-conversion or speed change","Generate a filename, minus the file extension","","","","","","","","","","","Validates (and attempts to fix) a phrase package. Returns …","","","A simple struct which allows a property on toml to be …","","","","","","","","","","","",""],"i":[0,0,0,0,0,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,0,2,2,2,2,2,2,2,2,2,2,2,2],"f":[[[["usize",15]],["string",3]],null,[[["str",15]],["string",3]],null,null,[[]],[[]],[[],["result",4]],[[],["string",3]],[[],["string",3]],null,[[]],[[]],[[],["smallvec",3]],null,[[],["smallvec",3]],null,[[],["result",4]],[[],["result",4]],[[],["typeid",3]],[[["config",3]],["result",4,[["string",3]]]],[[]],null,null,[[]],[[]],[[]],[[]],[[]],[[],["smallvec",3]],[[],["smallvec",3]],[[["str",15],["str",15],["pathtype",4],["pathbuf",3]]],[[],["result",4]],[[],["result",4]],[[],["typeid",3]],[[]]],"p":[[3,"PhrasePackage"],[3,"AlteredToml"]]}\
}');
if (window.initSearch) {window.initSearch(searchIndex)};