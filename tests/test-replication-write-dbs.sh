#!/bin/bash
trap "kill 0" EXIT

echo "Starting the primary"
primaryHttpAddress="127.0.0.1:9092"
primaryTcpAddress="127.0.0.1:3017"
secoundary1HttpAddress="127.0.0.1:9093"
secoundary2HttpAddress="127.0.0.1:9094"
cargo build
RUST_BACKTRACE=1 ./target/debug/nun-db --user mateus -p mateus start --http-address "$primaryHttpAddress" --tcp-address "$primaryTcpAddress" --ws-address "127.0.0.1:3058">primary.log&
PRIMARY_PID=$!

echo "Starting secoundary 1"

RUST_BACKTRACE=1 ./target/debug/nun-db --user mateus -p mateus start --http-address "$secoundary1HttpAddress" --tcp-address "127.0.0.1:3016" --ws-address "127.0.0.1:3057">secoundary.log&
SECOUNDARY_PID=$!


echo "Starting secoundary 2"

./target/debug/nun-db --user mateus -p mateus start --http-address "$secoundary2HttpAddress" --tcp-address "127.0.0.1:3018" --ws-address "127.0.0.1:3059">secoundary.2.log&
SECOUNDARY_2_PID=$!

sleep 1
sleep 2

echo "Will Connect the secoundaries to the primary"
#electionResult=$(curl -s -X "POST" "$primaryHttpAddress" -d "auth mateus mateus; election win")
echo "Election result: $electionResult"
joinResult=$(curl -s -X "POST" "$primaryHttpAddress" -d "auth mateus mateus; join 127.0.0.1:3016")
echo "Join 1 done"

clusterStatePrimary=$(curl -s -X "POST" "$primaryHttpAddress" -d "auth mateus mateus; cluster-state;")
echo "Final Primary: $clusterStatePrimary"

joinResult=$(curl -s -X "POST" "$primaryHttpAddress" -d "auth mateus mateus; join 127.0.0.1:3018")
echo "Join 2 done"
sleep 1
clusterStatePrimary=$(curl -s -X "POST" "$primaryHttpAddress" -d "auth mateus mateus; cluster-state;")
echo "Final Primary: $clusterStatePrimary"
sleep 5
clusterStateSecoundary=$(curl -s -X "POST" "$secoundary1HttpAddress" -d "auth mateus mateus; cluster-state;")
echo "Final Secoundary: $clusterStateSecoundary"

clusterStateSecoundary2=$(curl -s -X "POST" "$secoundary2HttpAddress" -d "auth mateus mateus; cluster-state;")
echo "Final Secoundary2: $clusterStateSecoundary2"
sleep 1
curl -s -X "POST" "$primaryHttpAddress" -d "auth mateus mateus; create-db test-db test-db-key;"

echo "Will test write in the secoundary read from primary"


sleep 3
start_time="$(date -u +%s)"

for i in {1..2000}
do
	echo "Set in the secoundary"
	r=$(curl -s -X "POST" "$secoundary1HttpAddress" -d "use-db test-db test-db-key; set state jose-$i-1;")
	echo "Read from the primary and secoundary"
	get_result=$(curl -s -X "POST" "$primaryHttpAddress" -d "use-db test-db test-db-key; get state")
	get_result2=$(curl -s -X "POST" "$secoundary2HttpAddress" -d "use-db test-db test-db-key; get state")
	if [ "$get_result" != "empty;value jose-$i-1" ]; then
		echo "Invalid value value in the primary: $get_result $i"
		exit 2
	else
		if [ "$get_result2" != "empty;value jose-$i-1" ]; then
			echo "Invalid value value in the secoundary 2: $get_result $i"
			exit 3
		else
			echo "Request $i Ok"
		fi 
	fi
done
end_time="$(date -u +%s)"
elapsed="$(($end_time-$start_time))"
echo "Total of $elapsed seconds elapsed for process"

exit 0

