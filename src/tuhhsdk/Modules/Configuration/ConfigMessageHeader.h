#pragma once

#include <stdint.h>
#include <string.h>

/**
 * This is the header portion of a network config message.
 * It gives information on how to process the body of the message.
 *
 * @author Robert Oehlmann
 */
struct ConfigMessageHeader
{
  ConfigMessageHeader()
  {
    strncpy(header, "CONF", sizeof header);
    version = 1;
  }

  //==============

  char header[4]; //"CONF"

  //=== 32bit ====

  uint8_t version;

  uint8_t msgType;
  uint16_t msgLength;

  //=== 32bit ====
};

/**
 * Possible types for the config message
 */
enum ConfigMessageType
{
  CM_SET = 0,
  CM_GET_MOUNTS = 1,
  CM_GET_KEYS = 2,
  CM_SAVE = 3,
  CM_SEND_KEYS = 4,
  CM_SEND_MOUNTS = 5
};
