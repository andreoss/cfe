Feature: Signing in
  A board tells somebody who they are, refuses a wrong password,
  and lets them leave again.

  This is asked last of all, on purpose, and the order is set out where
  the scenarios are gathered rather than left to chance. A board may answer a wrong
  password by demanding proof that the caller is human, for a good
  while afterwards and for everyone at that address, so anything that
  needs a successful sign-in has to be asked before it.

  Scenario: Somebody who has not signed in is nobody
    Given the board is answering
    Then the board does not know who they are

  Scenario: An administrator signs in
    Given the board is answering
    When an administrator signs in
    Then the board knows who they are

  Scenario: A reader signs in
    Given the board is answering
    When a reader signs in
    Then the board knows who they are

  Scenario: Signing out is remembered
    Given the board is answering
    When a reader signs in
    And they sign out
    Then the board does not know who they are

  Scenario: A wrong password is refused
    Given the board is answering
    When a reader signs in with the wrong password
    Then the board does not know who they are
