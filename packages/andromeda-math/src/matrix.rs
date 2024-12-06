use andromeda_std::{amp::AndrAddr, andr_exec, andr_instantiate, andr_query, error::ContractError};
use cosmwasm_schema::{cw_serde, QueryResponses};

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    pub authorized_operator_addresses: Option<Vec<AndrAddr>>,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    StoreMatrix { key: Option<String>, data: Matrix },
    DeleteMatrix { key: Option<String> },
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetMatrixResponse)]
    GetMatrix { key: Option<String> },
    #[returns(Vec<String>)]
    AllKeys {},
    #[returns(Vec<String>)]
    OwnerKeys { owner: AndrAddr },
}

#[cw_serde]
pub struct GetMatrixResponse {
    pub key: String,
    pub data: Matrix,
}

#[cw_serde]
pub struct Matrix(pub Vec<Vec<i64>>);

impl Matrix {
    pub fn validate_matrix(&self) -> Result<(), ContractError> {
        let row_length = self.0.first().map_or(0, |row| row.len());
        if !self.0.iter().all(|row| row.len() == row_length) {
            return Err(ContractError::CustomError {
                msg: "All rows in the matrix must have the same number of columns".to_string(),
            });
        }
        Ok(())
    }

    pub fn validate_add_sub(&self, other: &Matrix) -> Result<(), ContractError> {
        if self.0.len() != other.0.len() || self.0[0].len() != other.0[0].len() {
            return Err(ContractError::CustomError {
                msg: "Can not add or sub this matrix".to_string(),
            });
        }
        Ok(())
    }

    pub fn validate_mul(&self, other: &Matrix) -> Result<(), ContractError> {
        if self.0[0].len() != other.0.len() {
            return Err(ContractError::CustomError {
                msg: "Can not multiply this matrix".to_string(),
            });
        }
        Ok(())
    }

    pub fn add(&self, other: &Matrix) -> Result<Matrix, ContractError> {
        self.validate_add_sub(other)?;

        let matrix_data = self
            .0
            .iter()
            .zip(&other.0)
            .map(|(row_a, row_b)| {
                row_a
                    .iter()
                    .zip(row_b)
                    .map(|(a, b)| a.checked_add(*b).unwrap())
                    .collect()
            })
            .collect();

        Ok(Matrix(matrix_data))
    }

    pub fn sub(&self, other: &Matrix) -> Result<Matrix, ContractError> {
        self.validate_add_sub(other)?;

        let matrix_data = self
            .0
            .iter()
            .zip(&other.0)
            .map(|(row_a, row_b)| {
                row_a
                    .iter()
                    .zip(row_b)
                    .map(|(a, b)| a.checked_sub(*b).unwrap())
                    .collect()
            })
            .collect();

        Ok(Matrix(matrix_data))
    }

    #[allow(clippy::needless_range_loop)]
    pub fn mul(&self, other: &Matrix) -> Result<Matrix, ContractError> {
        self.validate_mul(other)?;

        let rows = self.0.len();
        let cols = other.0[0].len();
        let mut data = vec![vec![0_i64; cols]; rows];

        for i in 0..rows {
            for j in 0..cols {
                for k in 0..self.0[0].len() {
                    match self.0[i][k].checked_mul(other.0[k][j]) {
                        Some(val) => data[i][j] += val,
                        None => return Err(ContractError::Overflow {}),
                    }
                }
            }
        }

        Ok(Matrix(data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_matrix_valid() {
        // Test valid matrix with equal row lengths
        let matrix = Matrix(vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]]);

        assert!(matrix.validate_matrix().is_ok());
    }

    #[test]
    fn test_validate_matrix_invalid() {
        // Test invalid matrix with unequal row lengths
        let matrix = Matrix(vec![
            vec![1, 2, 3],
            vec![4, 5], // Invalid row length
            vec![7, 8, 9],
        ]);

        let result = matrix.validate_matrix();
        assert!(result.is_err());
        if let Err(ContractError::CustomError { msg }) = result {
            assert_eq!(
                msg,
                "All rows in the matrix must have the same number of columns".to_string()
            );
        } else {
            panic!("Expected CustomError, got something else");
        }
    }

    #[test]
    fn test_validate_add_sub_valid() {
        // Test valid matrices for addition and subtraction
        let matrix_a = Matrix(vec![vec![1, 2, 3], vec![4, 5, 6]]);

        let matrix_b = Matrix(vec![vec![7, 8, 9], vec![10, 11, 12]]);

        assert!(matrix_a.validate_add_sub(&matrix_b).is_ok());

        // Test addition result
        let result = matrix_a.add(&matrix_b).unwrap();
        assert_eq!(result.0, vec![vec![8, 10, 12], vec![14, 16, 18]]);

        // Test subtraction result
        let result = matrix_a.sub(&matrix_b).unwrap();
        assert_eq!(result.0, vec![vec![-6, -6, -6], vec![-6, -6, -6]]);
    }

    #[test]
    fn test_validate_add_sub_invalid() {
        // Test invalid matrices for addition and subtraction
        let matrix_a = Matrix(vec![vec![1, 2], vec![3, 4]]);

        let matrix_b = Matrix(vec![vec![5, 6, 7], vec![8, 9, 10]]);

        let result = matrix_a.validate_add_sub(&matrix_b);
        assert!(result.is_err());
        if let Err(ContractError::CustomError { msg }) = result {
            assert_eq!(msg, "Can not add or sub this matrix".to_string());
        } else {
            panic!("Expected CustomError, got something else");
        }
    }

    #[test]
    fn test_validate_mul_valid() {
        // Test valid matrices for multiplication
        let matrix_a = Matrix(vec![vec![1, 2, 3], vec![4, 5, 6]]);

        let matrix_b = Matrix(vec![vec![7, 8], vec![9, 10], vec![11, 12]]);

        assert!(matrix_a.validate_mul(&matrix_b).is_ok());

        // Test multiplication result
        let result = matrix_a.mul(&matrix_b).unwrap();
        assert_eq!(result.0, vec![vec![58, 64], vec![139, 154]]);
    }

    #[test]
    fn test_validate_mul_invalid() {
        // Test invalid matrices for multiplication
        let matrix_a = Matrix(vec![vec![1, 2], vec![3, 4]]);

        let matrix_b = Matrix(vec![vec![5, 6]]);

        let result = matrix_a.validate_mul(&matrix_b);
        assert!(result.is_err());
        if let Err(ContractError::CustomError { msg }) = result {
            assert_eq!(msg, "Can not multiply this matrix".to_string());
        } else {
            panic!("Expected CustomError, got something else");
        }
    }
}
