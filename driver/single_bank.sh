#!/usr/bin/env bash
# Clear DBANK option bit on STM32G474VETx (dual-bank -> single-bank).
set -euo pipefail

CHIP="STM32G474VETx"

FLASH_KEYR=0x40022008
FLASH_OPTKEYR=0x4002200C
FLASH_SR=0x40022010
FLASH_CR=0x40022014
FLASH_OPTR=0x40022020

KEY1=0x45670123
KEY2=0xCDEF89AB
OPTKEY1=0x08192A3B
OPTKEY2=0x4C5D6E7F

CR_LOCK=31
CR_OPTLOCK=30
CR_OPTSTRT=17
CR_OBL_LAUNCH=27
SR_BSY=16
SR_EOP=0
OPTR_DBANK=22

read32() { echo "0x$(probe-rs read b32 "$1" 1 --chip "$CHIP")"; }
write32() { probe-rs write b32 "$1" "$2" --chip "$CHIP" >/dev/null; }
bit() { echo $((($1 >> $2) & 1)); }

cr=$(read32 "$FLASH_CR")
if [[ $(bit "$cr" $CR_LOCK) -eq 1 ]]; then
    write32 "$FLASH_KEYR" "$KEY1"
    write32 "$FLASH_KEYR" "$KEY2"
fi

cr=$(read32 "$FLASH_CR")
if [[ $(bit "$cr" $CR_OPTLOCK) -eq 1 ]]; then
    write32 "$FLASH_OPTKEYR" "$OPTKEY1"
    write32 "$FLASH_OPTKEYR" "$OPTKEY2"
fi

optr=$(read32 "$FLASH_OPTR")
if [[ $(bit "$optr" $OPTR_DBANK) -eq 0 ]]; then
    echo "Already single-bank."
    exit 0
fi

write32 "$FLASH_OPTR" "$(printf '0x%08X' $((optr & ~(1 << OPTR_DBANK))))"

cr=$(read32 "$FLASH_CR")
write32 "$FLASH_CR" "$(printf '0x%08X' $((cr | (1 << CR_OPTSTRT))))"

while [[ $(bit "$(read32 "$FLASH_SR")" $SR_BSY) -eq 1 ]]; do sleep 0.05; done

sr=$(read32 "$FLASH_SR")
[[ $(bit "$sr" $SR_EOP) -eq 1 ]] && write32 "$FLASH_SR" "$(printf '0x%08X' $((1 << SR_EOP)))"

optr=$(read32 "$FLASH_OPTR")
if [[ $(bit "$optr" $OPTR_DBANK) -ne 0 ]]; then
    echo "ERROR: DBANK still set." >&2
    exit 1
fi

cr=$(read32 "$FLASH_CR")
write32 "$FLASH_CR" "$(printf '0x%08X' $((cr | (1 << CR_OBL_LAUNCH))))"
