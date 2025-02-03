use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    program::invoke_signed,
    pubkey::Pubkey,
    system_instruction::create_account,
    system_program::ID as SYSTEM_PROGRAM_ID,
    sysvar::rent::Rent,
    sysvar::Sysvar,
};

solana_program::declare_id!("3VG4GdkTVETFrpmfCMVoGr4G73rh4hkPrB9vQUKyPNx5");
entrypoint!(process_instruction);

#[derive(BorshSerialize, BorshDeserialize)]
pub enum TodoInstruction {
    CreateTask {
        id: u64,
        title: [u8; 64],
        description: [u8; 64],
        bump: u8,
    },
    UpdateTask {
        id: u64,
        title: Option<[u8; 64]>,
        description: Option<[u8; 64]>,
    },
    DeleteTask {
        id: u64,
    },
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Task {
    title: [u8; 64],       // 64 +
    description: [u8; 64], // 64 +
    authority: Pubkey,     // 32 +
    id: u64,               // 8 +
    bump: u8,              // 1 = 169 bytes
}

impl Task {
    const LEN: usize = 169; // 64 + 64 + 32 + 8 + 1
    const TAG: &'static str = "task";

    pub fn new(
        id: u64,
        title: [u8; 64],
        description: [u8; 64],
        authority: Pubkey,
        bump: u8,
    ) -> Self {
        Self {
            id,
            title,
            description,
            authority,
            bump,
        }
    }

    pub fn create_pda(program_id: &Pubkey, id: u64, authority: &Pubkey, bump: u8) -> Pubkey {
        // expects a valid set of seeds
        Pubkey::create_program_address(
            &[
                Self::TAG.as_bytes(),
                &id.to_le_bytes(),
                authority.as_ref(),
                &[bump],
            ],
            program_id,
        )
        .expect("Invalid Seeds")
    }
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = TodoInstruction::try_from_slice(instruction_data)?;

    match instruction {
        // Accounts.
        // - 1. [WRITE, SIGNER]   Authority and Payer.
        // - 2. [WRITE]           Task.
        // - 3. []                System Program.
        TodoInstruction::CreateTask {
            id,
            title,
            description,
            bump,
        } => {
            let accounts_iter = &mut accounts.iter();
            let authority = next_account_info(accounts_iter)?;
            let task = next_account_info(accounts_iter)?;
            let system_program = next_account_info(accounts_iter)?;

            // Simple assert validations.
            assert!(authority.is_signer);
            assert!(authority.is_writable);
            assert!(!task.is_signer); // NOT
            assert!(task.is_writable);
            assert!(*system_program.key == SYSTEM_PROGRAM_ID);

            // PDA: Program Derived Address.
            let (task_pda, task_bump) = Pubkey::find_program_address(
                &[
                    Task::TAG.as_bytes(),
                    &id.to_le_bytes(),
                    authority.key.as_ref(),
                ],
                program_id,
            );

            // Business Logic.

            // Create Task account with System Program.
            let task_rent = Rent::get()?.minimum_balance(Task::LEN);
            let task_space: u64 = Task::LEN as u64;
            invoke_signed(
                &create_account(authority.key, &task_pda, task_rent, task_space, program_id),
                &[authority.clone(), task.clone()],
                &[&[
                    Task::TAG.as_bytes(),
                    &id.to_le_bytes(),
                    authority.key.as_ref(),
                    &[task_bump],
                ]],
            )?;

            let mut task_raw_bytes = task.try_borrow_mut_data()?;
            let mut task_data = Task::try_from_slice(&task_raw_bytes)?;
            task_data.title = [0u8; 64];
            task_data.description = [0u8; 64];
            task_data.title = title;
            task_data.description = description;
            task_data.authority = *authority.key;
            task_data.id = id;
            task_data.bump = bump;

            task_data.serialize(&mut &mut task_raw_bytes[..])?;
        }

        // Accounts
        // - 1. [WRITE, SIGNER]   Authority and Payer.
        // - 2. [WRITE]           Task.
        TodoInstruction::UpdateTask {
            id,
            title,
            description,
        } => {
            let accounts_iter = &mut accounts.iter();
            let authority = next_account_info(accounts_iter)?;
            let task = next_account_info(accounts_iter)?;

            assert!(authority.is_signer);
            assert!(authority.is_writable);
            assert!(!task.is_signer); // NOT
            assert!(task.is_writable);
            assert!(title.is_some() || description.is_some());

            let mut task_raw_bytes = task.try_borrow_mut_data()?;
            let mut task_data = Task::try_from_slice(&task_raw_bytes)?;

            assert!(*authority.key == task_data.authority);
            assert!(id == task_data.id);

            let task_pda = Task::create_pda(
                &program_id,
                task_data.id,
                &task_data.authority,
                task_data.bump,
            );

            assert!(*task.key == task_pda);

            if title.is_some() {
                task_data.title = [0u8; 64];
                task_data.title = title.unwrap();
            }
            if description.is_some() {
                task_data.description = [0u8; 64];
                task_data.description = description.unwrap();
            }

            task_data.serialize(&mut &mut task_raw_bytes[..])?;
        }

        // Accounts.
        // - 1. [WRITE, SIGNER]   Authority and Payer.
        // - 2. [WRITE]           Task.
        TodoInstruction::DeleteTask { id } => {
            let accounts_iter = &mut accounts.iter();
            let authority = next_account_info(accounts_iter)?;
            let task = next_account_info(accounts_iter)?;

            assert!(authority.is_signer);
            assert!(authority.is_writable);
            assert!(!task.is_signer); // NOT
            assert!(task.is_writable);

            let mut task_raw_bytes = task.try_borrow_mut_data()?;
            let task_data = Task::try_from_slice(&task_raw_bytes)?;

            assert!(*authority.key == task_data.authority);
            assert!(id == task_data.id);

            let task_pda = Task::create_pda(
                &program_id,
                task_data.id,
                &task_data.authority,
                task_data.bump,
            );

            assert!(*task.key == task_pda);

            // Close the Task (PDA) account by Zeroing.
            task_raw_bytes.fill(0);

            let task_lamports = task.lamports();
            let authority_lamports = authority.lamports();

            // Direct transfer Task (PDA) lamports into authority.
            // NOTE: Direct transfer is okay since Task is a PDA owned by authority.
            **authority.try_borrow_mut_lamports()? = authority_lamports
                .checked_add(task_lamports) // None if overflow.
                .unwrap();

            // Zero out Task (PDA) lamports.
            //
            // Runtime eventually cleans this account up due to 0 rent;
            **task.try_borrow_mut_lamports()? = 0;
        }
    };

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program_test::*;
    use solana_sdk::{
        account::Account,
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        system_program::ID as SYSTEM_PROGRAM_ID,
        transaction::Transaction,
    };

    fn get_env() -> ([u8; 64], [u8; 64], [u8; 64], [u8; 64]) {
        let title_str = "My task";
        let description_str = "My task description";

        let mut title: [u8; 64] = [0u8; 64];
        title[..title_str.len()].copy_from_slice(title_str.as_bytes());
        let mut description: [u8; 64] = [0u8; 64];
        description[..description_str.len()].copy_from_slice(description_str.as_bytes());

        let new_title_str = "My new task";
        let new_description_str = "My new task description";

        let mut new_title: [u8; 64] = [0u8; 64];
        new_title[..new_title_str.len()].copy_from_slice(new_title_str.as_bytes());
        let mut new_description: [u8; 64] = [0u8; 64];
        new_description[..new_description_str.len()]
            .copy_from_slice(new_description_str.as_bytes());

        (title, description, new_title, new_description)
    }

    //------------------------------------------------------------
    // Testing Happy Path
    //------------------------------------------------------------

    #[tokio::test]
    async fn test_create_task() {
        let program_id = Pubkey::new_unique();
        let mut test = ProgramTest::default();
        test.add_program("todo", program_id, None);
        let ctx = test.start_with_context().await;

        let id: u64 = 1;
        let (title, description, _, _) = get_env();

        let (task_pda, task_bump) = Pubkey::find_program_address(
            &[
                Task::TAG.as_bytes(),
                &id.to_le_bytes(),
                ctx.payer.pubkey().as_ref(),
            ],
            &program_id,
        );

        let create_task_ix = TodoInstruction::CreateTask {
            id,
            title,
            description,
            bump: task_bump,
        };

        let mut create_task_ix_data = Vec::new();
        create_task_ix.serialize(&mut create_task_ix_data).unwrap();

        let transaction = Transaction::new_signed_with_payer(
            &[Instruction {
                program_id,
                accounts: vec![
                    AccountMeta::new(ctx.payer.pubkey(), true),
                    AccountMeta::new(task_pda, false),
                    AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
                ],
                data: create_task_ix_data.clone(),
            }],
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer.insecure_clone()],
            ctx.last_blockhash,
        );

        // send transaction
        ctx.banks_client
            .process_transaction(transaction)
            .await
            .unwrap();

        // confirm state
        let task = ctx
            .banks_client
            .get_account_data_with_borsh::<Task>(task_pda)
            .await
            .unwrap();

        assert_eq!(id, task.id);
        assert_eq!(title, task.title);
        assert_eq!(description, task.description);
        assert_eq!(ctx.payer.pubkey(), task.authority);
        assert_eq!(task_bump, task.bump);
    }

    #[tokio::test]
    async fn test_update_task() {
        let program_id = Pubkey::new_unique();
        let id: u64 = 1;
        let (title, description, new_title, new_description) = get_env();
        let authority_keypair = Keypair::new();
        let authority = authority_keypair.pubkey();

        let (task_pda, task_bump) = Pubkey::find_program_address(
            &[Task::TAG.as_bytes(), &id.to_le_bytes(), authority.as_ref()],
            &program_id,
        );

        // Inject Account that simulates CreateTask.
        let old_task = Task::new(id, title, description, authority, task_bump);
        let mut account_data: Vec<u8> = Vec::new();
        old_task.serialize(&mut account_data).unwrap();
        let injected_task_account = Account {
            lamports: u32::MAX as u64,
            owner: program_id,
            executable: false,
            rent_epoch: Default::default(),
            data: account_data,
        };

        // Inject account into Test.
        let mut test = ProgramTest::default();
        test.add_program("todo", program_id, None);
        test.add_account(task_pda, injected_task_account);
        let ctx = test.start_with_context().await;

        let update_task_ix = TodoInstruction::UpdateTask {
            id,
            title: Some(new_title),
            description: Some(new_description),
        };

        let mut update_task_ix_data = Vec::new();
        update_task_ix.serialize(&mut update_task_ix_data).unwrap();

        let transaction = Transaction::new_signed_with_payer(
            &[Instruction {
                program_id,
                accounts: vec![
                    AccountMeta::new(authority, true),
                    AccountMeta::new(task_pda, false),
                ],
                data: update_task_ix_data.clone(),
            }],
            Some(&ctx.payer.pubkey()),
            &[&authority_keypair, &ctx.payer.insecure_clone()],
            ctx.last_blockhash,
        );

        // send transaction
        ctx.banks_client
            .process_transaction(transaction)
            .await
            .unwrap();

        // confirm state
        let task = ctx
            .banks_client
            .get_account_data_with_borsh::<Task>(task_pda)
            .await
            .unwrap();

        assert_eq!(id, task.id);
        assert_eq!(new_title, task.title);
        assert_eq!(new_description, task.description);
        assert_eq!(authority, task.authority);
        assert_eq!(task_bump, task.bump);
    }

    #[tokio::test]
    async fn test_delete_task() {
        let program_id = Pubkey::new_unique();
        let id: u64 = 1;
        let (title, description, _, _) = get_env();
        let authority_keypair = Keypair::new();
        let authority = authority_keypair.pubkey();

        let (task_pda, task_bump) = Pubkey::find_program_address(
            &[Task::TAG.as_bytes(), &id.to_le_bytes(), authority.as_ref()],
            &program_id,
        );

        // Inject Account that simulates CreateTask.
        let old_task = Task::new(id, title, description, authority, task_bump);
        let mut account_data: Vec<u8> = Vec::new();
        old_task.serialize(&mut account_data).unwrap();
        let injected_task_account = Account {
            lamports: u32::MAX as u64,
            owner: program_id,
            executable: false,
            rent_epoch: Default::default(),
            data: account_data,
        };

        // Inject account into Test.
        let mut test = ProgramTest::default();
        test.add_program("todo", program_id, None);
        test.add_account(task_pda, injected_task_account);
        let ctx = test.start_with_context().await;

        let delete_task_ix = TodoInstruction::DeleteTask { id };

        let mut delete_task_ix_data = Vec::new();
        delete_task_ix.serialize(&mut delete_task_ix_data).unwrap();

        let transaction = Transaction::new_signed_with_payer(
            &[Instruction {
                program_id,
                accounts: vec![
                    AccountMeta::new(authority, true),
                    AccountMeta::new(task_pda, false),
                ],
                data: delete_task_ix_data.clone(),
            }],
            Some(&ctx.payer.pubkey()),
            &[&authority_keypair, &ctx.payer.insecure_clone()],
            ctx.last_blockhash,
        );

        // send transaction
        ctx.banks_client
            .process_transaction(transaction)
            .await
            .unwrap();

        // confirm state
        let task_account = ctx.banks_client.get_account(task_pda).await.unwrap();

        assert!(task_account.is_none());
    }

    //------------------------------------------------------------
    // Other Testing
    //------------------------------------------------------------
    #[tokio::test]
    #[should_panic]
    async fn test_update_task_with_none() {
        let program_id = Pubkey::new_unique();
        let id: u64 = 1;
        let (title, description, _, _) = get_env();
        let authority_keypair = Keypair::new();
        let authority = authority_keypair.pubkey();

        let (task_pda, task_bump) = Pubkey::find_program_address(
            &[Task::TAG.as_bytes(), &id.to_le_bytes(), authority.as_ref()],
            &program_id,
        );

        // Inject Account that simulates CreateTask.
        let old_task = Task::new(id, title, description, authority, task_bump);
        let mut account_data: Vec<u8> = Vec::new();
        old_task.serialize(&mut account_data).unwrap();
        let injected_task_account = Account {
            lamports: u32::MAX as u64,
            owner: program_id,
            executable: false,
            rent_epoch: Default::default(),
            data: account_data,
        };

        // Inject account into Test.
        let mut test = ProgramTest::default();
        test.add_program("todo", program_id, None);
        test.add_account(task_pda, injected_task_account);
        let ctx = test.start_with_context().await;

        let update_task_ix = TodoInstruction::UpdateTask {
            id,
            title: None,
            description: None,
        };

        let mut update_task_ix_data = Vec::new();
        update_task_ix.serialize(&mut update_task_ix_data).unwrap();

        let transaction = Transaction::new_signed_with_payer(
            &[Instruction {
                program_id,
                accounts: vec![
                    AccountMeta::new(authority, true),
                    AccountMeta::new(task_pda, false),
                ],
                data: update_task_ix_data.clone(),
            }],
            Some(&ctx.payer.pubkey()),
            &[&authority_keypair, &ctx.payer.insecure_clone()],
            ctx.last_blockhash,
        );

        // send transaction
        ctx.banks_client
            .process_transaction(transaction)
            .await
            .unwrap();
    }
}
