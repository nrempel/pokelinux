//! Recovered vendor request transport. Successful one-way submission is not
//! modem acceptance. Session lifetime, discovery and retries belong to the owner.
use omarchy_ims::requests::{RequestPart, RequestParts};
use rsbinder::{BinderResult, FLAG_ONEWAY, Parcel, Result, SIBinder, Status, StatusCode};

pub const RADIO_DESCRIPTOR: &str =
    "vendor.samsung_slsi.telephony.hardware.radioExternal.IOemSlsiRadioExternal";
pub const INDICATION_DESCRIPTOR: &str =
    "vendor.samsung_slsi.telephony.hardware.radioExternal.IOemSlsiRadioExternalInd";

pub struct RadioTransport(SIBinder);
#[derive(Debug)]
pub struct SubmissionError {
    pub source: StatusCode,
    /// Parts submitted before failure; their remote execution remains unknown.
    pub submitted_parts: usize,
}
impl RadioTransport {
    pub fn from_binder(binder: SIBinder) -> Result<Self> {
        if !binder.is_remote() || binder.descriptor() != RADIO_DESCRIPTOR {
            return Err(StatusCode::BadType);
        }
        Ok(Self(binder))
    }
    fn input(&self) -> Result<Parcel> {
        self.0
            .as_remote()
            .ok_or(StatusCode::BadType)?
            .prepare_transact(true)
    }
    pub fn register(
        &self,
        response: &SIBinder,
        indication: &SIBinder,
        client: &str,
    ) -> BinderResult<i32> {
        if response.descriptor() != crate::receiver::RESPONSE_DESCRIPTOR
            || indication.descriptor() != INDICATION_DESCRIPTOR
        {
            return Err(StatusCode::BadType.into());
        }
        let mut data = self.input()?;
        data.write(response)?;
        data.write(indication)?;
        data.write(client)?;
        let mut reply = self
            .0
            .as_remote()
            .ok_or(StatusCode::BadType)?
            .submit_transact(5, &data, 0)?
            .ok_or(StatusCode::UnexpectedNull)?;
        let status: Status = reply.read()?;
        if !status.is_ok() {
            return Err(status);
        }
        let id = reply.read_i32()?;
        if reply.data_avail() != 0 {
            return Err(StatusCode::BadValue.into());
        }
        Ok(id)
    }
    pub fn clear(&self, client_id: i32) -> Result<()> {
        let mut data = self.input()?;
        data.write_i32(client_id)?;
        self.0
            .as_remote()
            .ok_or(StatusCode::BadType)?
            .submit_transact(1, &data, FLAG_ONEWAY)?;
        Ok(())
    }
    /// Submit once, in order, without retries. Any failure leaves the complete
    /// operation uncertain; the caller must not blindly replay it.
    pub fn send(
        &self,
        serial: i32,
        client_id: i32,
        request: i32,
        slot: i32,
        payload: &[u8],
    ) -> std::result::Result<(), SubmissionError> {
        let parts = RequestParts::new(payload).map_err(|_| SubmissionError {
            source: StatusCode::BadValue,
            submitted_parts: 0,
        })?;
        for (submitted_parts, part) in parts.enumerate() {
            let result = (|| -> Result<()> {
                let mut data = self.input()?;
                for value in [serial, client_id, request, slot] {
                    data.write_i32(value)?;
                }
                let code = match part {
                    RequestPart::Raw(bytes) => {
                        data.write(bytes)?;
                        2
                    }
                    RequestPart::Segment {
                        index,
                        total_length,
                        payload,
                    } => {
                        data.write(payload)?;
                        data.write_i32(index)?;
                        data.write_i32(total_length)?;
                        3
                    }
                };
                self.0
                    .as_remote()
                    .ok_or(StatusCode::BadType)?
                    .submit_transact(code, &data, FLAG_ONEWAY)?;
                Ok(())
            })();
            if let Err(source) = result {
                return Err(SubmissionError {
                    source,
                    submitted_parts,
                });
            }
        }
        Ok(())
    }
}
