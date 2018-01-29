#include "Messaging.hpp"

Sender::Sender(DataQueue& queue)
  : queue_(queue)
{
}

bool Sender::send(const Message& msg)
{
  return queue_.push(msg);
}

const std::vector<std::type_index>& Sender::getRequested() const
{
  return requested_;
}

Receiver::Receiver(Sender& sender)
  : sender_(sender)
{
}

bool Receiver::receive(Message& msg)
{
  return sender_.queue_.pop(msg);
}

void Receiver::request(const std::type_index& type)
{
  sender_.requested_.push_back(type);
}

DuplexChannel::DuplexChannel()
  : a2b_()
  , b2a_()
  , a2b_sender_(a2b_)
  , a2b_receiver_(a2b_sender_)
  , b2a_sender_(b2a_)
  , b2a_receiver_(b2a_sender_)
{
}

Sender& DuplexChannel::getA2BSender()
{
  return a2b_sender_;
}

Receiver& DuplexChannel::getA2BReceiver()
{
  return a2b_receiver_;
}

Sender& DuplexChannel::getB2ASender()
{
  return b2a_sender_;
}

Receiver& DuplexChannel::getB2AReceiver()
{
  return b2a_receiver_;
}
