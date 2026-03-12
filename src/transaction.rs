use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};

/// Represents a transaction in the blockchain.
///
/// Each transaction consists of a unique identifier (derived from the transaction data)
/// and the actual transaction data.
///
/// The `Transaction` struct is serializable using Serde, enabling it to be easily
/// stored, transferred over the network, or included in a block.
///
/// # Example
///
/// ```
/// use your_crate::Transaction;
///
/// let tx = Transaction::new("Send 10 BTC to Alice".to_string());
/// println!("Transaction ID: {:?}", tx.get_id());
/// println!("Transaction Data: {}", tx.get_data());
/// ```
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Transaction {
    /// The unique identifier for the transaction.
    ///
    /// Generated using a SHA-256 hash of the transaction data.
    pub id: Vec<u8>,

    /// The data payload of the transaction (e.g., payment details, metadata).
    pub data: String,
}

impl Transaction {
    /// Creates a new transaction and generates its unique ID based on the provided data.
    ///
    /// # Arguments
    ///
    /// * `data` - The data payload for the transaction.
    ///
    /// # Returns
    ///
    /// A `Transaction` instance with a SHA-256 generated ID.
    ///
    /// # Example
    ///
    /// ```
    /// use your_crate::Transaction;
    ///
    /// let tx = Transaction::new("Send 5 BTC to Bob".to_string());
    /// assert_eq!(tx.get_data(), "Send 5 BTC to Bob");
    /// ```
    pub fn new(data: String) -> Transaction {
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let id = hasher.finalize().to_vec();

        Transaction { id, data }
    }

    /// Returns the unique identifier of the transaction.
    ///
    /// # Returns
    ///
    /// A `Vec<u8>` representing the SHA-256 hash of the transaction data.
    ///
    /// # Example
    ///
    /// ```
    /// use your_crate::Transaction;
    ///
    /// let tx = Transaction::new("Send 5 BTC".to_string());
    /// let id = tx.get_id();
    /// println!("Transaction ID: {:?}", id);
    /// ```
    pub fn get_id(&self) -> Vec<u8> {
        self.id.clone()
    }

    /// Returns the data payload of the transaction.
    ///
    /// # Returns
    ///
    /// A reference to the `String` containing the transaction's data.
    ///
    /// # Example
    ///
    /// ```
    /// use your_crate::Transaction;
    ///
    /// let tx = Transaction::new("Send 5 BTC".to_string());
    /// assert_eq!(tx.get_data(), "Send 5 BTC");
    /// ```
    #[allow(dead_code)]
    pub fn get_data(&self) -> &String {
        &self.data
    }
}
