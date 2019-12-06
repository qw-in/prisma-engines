#!/bin/bash
echo "{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"introspect\",\"params\":[{\"url\":\"file:../db/lift.db\"}]}" | ../target/debug/introspection-engine
# debug
# echo "{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"introspect\",\"params\":[{\"url\":\"file:${folder}${fileName}.db\"}]}" | ../target/debug/introspection-engine 



# do this for all .db files in the directory. overwrite old .prisma files, can use diff in git / source tree to see changes