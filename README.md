# esp32 firmware

## usage

1. Create a binary file for the configuration data that lives on the NVS partition

`cargo run --package cli write --output-file nvs.bin`

2. Write the NVS partition to the device's flash. Note the address being written to; it should match the partition table of the device

`espflash write-bin 0x9000 nvs.bin`

3. Write the firmware

`cd crates/relay-controller && cargo espflash flash --monitor --partition-table partitions.csv --chip esp32s3 -s 8mb --target-app-partition ota_0`

4. Export the firmware

`cargo espflash save-image --chip esp32s3 ~/n/nas/esp32/relay-controller/files/$(git rev-parse --short HEAD) --partition-table partitions.csv -s 8mb`

5. Update the running version

`git rev-parse --short HEAD > ~/n/nas/esp32/relay-controller/version`
