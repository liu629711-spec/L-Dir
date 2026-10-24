// ==== FUN_00456560 @ 00456560 ====

longlong FUN_00456560(longlong param_1)

{
  int iVar1;
  longlong lVar2;
  undefined8 local_res8 [4];
  
  local_res8[0] = 0;
  if (*(longlong *)(param_1 + 0x4e40) == 0) {
    lVar2 = 0;
  }
  else {
    lVar2 = param_1 + 0x4e38;
    iVar1 = FUN_004bfab4(lVar2,local_res8);
    if (iVar1 == 0) {
      iVar1 = FUN_004bfab4(param_1 + 0x5950,local_res8);
      if (iVar1 == 0) {
        iVar1 = FUN_004bfab4(param_1 + 0x6468,local_res8);
        if (iVar1 == 0) {
          iVar1 = FUN_004bfab4(param_1 + 0x6f80,local_res8);
          if (iVar1 != 0) {
            lVar2 = param_1 + 0x6f80;
          }
        }
        else {
          lVar2 = param_1 + 0x6468;
        }
      }
      else {
        lVar2 = param_1 + 0x5950;
      }
    }
  }
  return lVar2;
}


