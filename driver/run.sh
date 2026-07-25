#!/bin/bash
# Download and start the firmware before starting devis/rerun
probe-rs download --chip STM32G474VETx "$1"
probe-rs reset --chip STM32G474VETx
devis --chip STM32G474VETx "$1"
