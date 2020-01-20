#ifndef ALIVENESSTRANSMITTER_H
#define ALIVENESSTRANSMITTER_H

#include <cstdint>
#include <memory>

#include "Modules/Configuration/Configuration.h"


struct NaoInfo;

class AlivenessTransmitter
{
private:
  class Impl;

  std::unique_ptr<Impl> pimpl_;
  bool isTransmittingStarted_;

public:
  AlivenessTransmitter(const std::uint16_t& port, const NaoInfo& naoInfo, Configuration& config);
  ~AlivenessTransmitter();

  void startTransmitting();
  void stopTransmitting();
};

#endif // ALIVENESSTRANSMITTER_H
