#include "AlivenessTransmitter.h"
#include "AlivenessMessage.h"
#include "print.h"

#include "Hardware/RobotInterface.hpp"

#include <boost/asio.hpp>

#include <boost/array.hpp>
#include <boost/bind.hpp>
#include <boost/system/error_code.hpp>

// ASIO defines an macro  #define ERROR 0 which failes with our enum LogLevel::ERROR
#include "Definitions/windows_definition_fix.hpp"

#include <thread>

static auto const INTERVAL = boost::posix_time::millisec(1000);

class AlivenessTransmitter::Impl
{
private:
  boost::asio::io_service ioService_;

  boost::asio::ip::udp::endpoint broadcastEndpoint_;
  boost::asio::ip::udp::socket socket_;

  std::shared_ptr<std::thread> backgroundThread_;

  boost::asio::deadline_timer timer_;
  boost::array<char, sizeof(AlivenessMessage)> send_; ///< Send buffer.

  NaoInfo naoInfo_;
  Configuration& config_;

  void onTimerExpire(boost::system::error_code const& error);

public:
  Impl(const uint16_t& port, const NaoInfo& naoInfo, Configuration& config);
  ~Impl();

  void startTransmitTimer();
  void startBackgroundThread();
};

//================================
// Pimpl
//================================

AlivenessTransmitter::Impl::Impl(const uint16_t& port, const NaoInfo& naoInfo, Configuration& config)
  : ioService_()
  , broadcastEndpoint_()
  , socket_(ioService_)
  , timer_(ioService_)
  , naoInfo_(naoInfo)
  , config_(config)
{
  boost::asio::ip::udp::endpoint localEndpoint(boost::asio::ip::udp::v4(), port);

  broadcastEndpoint_.address(boost::asio::ip::address_v4::broadcast());
  broadcastEndpoint_.port(port);

  socket_.open(localEndpoint.protocol());
  socket_.set_option(boost::asio::ip::udp::socket::reuse_address(true));
  socket_.set_option(boost::asio::socket_base::broadcast(true));
  socket_.bind(localEndpoint);
}

AlivenessTransmitter::Impl::~Impl()
{
  ioService_.stop();
  backgroundThread_->join();
  socket_.close();
}

void AlivenessTransmitter::Impl::startBackgroundThread()
{
  backgroundThread_ = std::make_shared<std::thread>([this]() {
    // Run this thread until ioService_.stop() is called.
    //       boost::asio::io_service::work work( ioService_ );

    ioService_.run();
    print("Shutting down transceiver thread", LogLevel::INFO);
  });
}

void AlivenessTransmitter::Impl::startTransmitTimer()
{
  timer_.expires_from_now(INTERVAL);
  timer_.async_wait(boost::bind(&Impl::onTimerExpire, this, boost::asio::placeholders::error));
}

void AlivenessTransmitter::Impl::onTimerExpire(boost::system::error_code const& error)
{
  if (error)
  {
    print("AlivenessTransmitter timer error.", LogLevel::ERROR);
    return;
  }

  AlivenessMessage msg(naoInfo_.bodyName, naoInfo_.headName, config_);
  memcpy(send_.data(), &msg, send_.size() * sizeof send_[0]);

  try
  {
    socket_.send_to(boost::asio::buffer(send_), broadcastEndpoint_);
  }
  catch (boost::system::system_error&)
  {
    print("Error sending Aliveness Message!", LogLevel::ERROR);
  }

  // And cycle the timer.
  startTransmitTimer();
}

//================================
// AlivenessTransmitter
//================================


AlivenessTransmitter::AlivenessTransmitter(const std::uint16_t& port, const NaoInfo& naoInfo, Configuration& config)
  : isTransmittingStarted_(false)
{
  pimpl_ = std::make_unique<Impl>(port, naoInfo, config);
}

AlivenessTransmitter::~AlivenessTransmitter() = default;

void AlivenessTransmitter::startTransmitting()
{
  if (!isTransmittingStarted_)
  {
    isTransmittingStarted_ = true;

    pimpl_->startTransmitTimer();
    pimpl_->startBackgroundThread();
  }
  else
  {
    print("Aliveness Transmitter is already started!", LogLevel::WARNING);
  }
}
