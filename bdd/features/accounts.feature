Feature: Accounts
  An account has a name others can look it up by, and a standing that
  belongs to it. Somebody signed in is told which account they are.

  Scenario: An account that does not exist is not invented
    Given the board is answering
    When somebody looks up an account that does not exist
    Then the account is not found

  Scenario: An account keeps its name across a visit
    Given the board is answering
    When an administrator signs in
    And they look themselves up
    Then the account found is the one they signed in as

  Scenario: A moderator is an account like any other to look up
    Given the board is answering
    When somebody looks up the account of a moderator
    Then the account names itself
