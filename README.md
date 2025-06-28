# esp32 firmware

## usage

1. Create a binary file for the configuration data that lives on the NVS partition

`cargo run --package cli write --output-file nvs.bin`

2. Write the NVS partition to the device's flash. Note the address being written to; it should match the partition table of the device

`espflash write-bin 0x9000 nvs.bin`

3. Write the firmware

`cd crates/relay-controller && cargo espflash flash --monitor --partition-table partitions.csv`
