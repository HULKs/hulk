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
  //==============

  char header[4] = {'C', 'O', 'N', 'F'};

  //=== 32bit ====

  uint8_t version = 1;

  uint8_t msgType = 0;
  uint16_t msgLength = 0;

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
