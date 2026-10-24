// ==== FUN_0045a30c @ 0045a30c ====

undefined8 FUN_0045a30c(longlong param_1)

{
  uint uVar1;
  undefined4 *in_stack_00000028;
  
  uVar1 = (uint)(*(int *)(param_1 + 0x4e24) == 0);
  *(uint *)(param_1 + 0x4e24) = uVar1;
  if (uVar1 == 0) {
    FUN_00456c18(param_1);
  }
  else {
    FUN_00456b14(param_1);
  }
  *in_stack_00000028 = 1;
  return 0;
}


