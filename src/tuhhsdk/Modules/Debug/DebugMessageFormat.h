#pragma once

#include <stdint.h>
#include <string.h>

#include "Definitions/windows_definition_fix.hpp"

/**
 * This is the header portion of a debug message. It
 * gives hints on how to process the rest of a given
 * message.
 *
 * @author Finn Poppinga
 */
struct DebugMessageHeader
{
  DebugMessageHeader()
    : version()
    , msgType()
    , __padding()
    , msgLength()
  {
    strncpy(header, "DMSG", sizeof header);
    version = 1;
    __padding = 0;
  }

  //==============
  char header[4]; //"DMSG"
  //=== 32bit ====
  uint8_t version;
  uint8_t msgType;
  uint16_t __padding;
  //=== 32bit ====
  uint32_t msgLength;
  //=== 32bit ====
  uint32_t ___padding;
  //=== 32bit ====
};

/**
 * Possible types for the RC message
 */
enum DebugMessageType
{
  DM_SUBSCRIBE = 0,
  DM_UNSUBSCRIBE = 1,
  DM_UPDATE = 2,
  DM_REQUEST_LIST = 3,
  DM_LIST = 4,
  DM_SUBSCRIBE_BULK = 5,
  DM_IMAGE = 6
};
