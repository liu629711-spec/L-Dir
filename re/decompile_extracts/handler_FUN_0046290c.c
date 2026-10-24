// ==== FUN_0046290c @ 0046290c ====

ulonglong FUN_0046290c(longlong param_1,undefined8 param_2,ulonglong param_3,ulonglong param_4,
                      undefined4 *param_5)

{
  ulonglong uVar1;
  
  *param_5 = 1;
  uVar1 = (ulonglong)DAT_00551014;
  if ((uVar1 == param_3) && (uVar1 == param_4)) {
    DAT_00550f84 = 0;
    FUN_00461e54(param_1);
    PostMessageW(*(HWND *)(param_1 + 8),0x10,0,0);
    uVar1 = (ulonglong)DAT_00551014;
  }
  return uVar1;
}


